use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;

use crate::query::Page;
use crate::store::{KVCursor, KVEntry, KVRead, SortOrder, StoreError};

/// Entries read per page while scanning a locale.
const ENTRIES_PER_PAGE: usize = 1024;

/// How much of the reference locale's key set one locale translates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LocaleCoverage {
    /// Reference keys this locale translates with a non-empty value.
    pub present: usize,
    /// `reference_keys - present`.
    pub missing: usize,
    /// `present` over [`CoverageReport::reference_keys`]; `1.0` when the
    /// reference translates no keys.
    pub ratio: f64,
}

/// How much of the reference locale every other locale translates.
///
/// Coverage is measured inside one store, so it answers "which languages are
/// incomplete in this version" rather than "what changed between versions". The
/// reference locale - the source language - defines the key universe; the other
/// locales are measured against it and a locale that exists only in the
/// reference reports 0%. The reference itself is not a row. A key counts as
/// translated only when the locale holds a non-empty value for it, and it only
/// enters the universe when the reference holds a non-empty value for it: an
/// empty source string defines nothing to translate.
///
/// Missing keys are a count here; the keys themselves come from
/// [`Coverage::missing_keys`], which pages instead of materialising a list that
/// can be as large as the whole project.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoverageReport {
    /// The locale that defines the key universe.
    pub reference: String,
    /// Keys the reference locale translates, i.e. holds with a non-empty value.
    pub reference_keys: usize,
    /// `Σpresent / (locales × reference_keys)`; `1.0` when there is nothing to
    /// translate.
    pub overall: f64,
    pub per_locale: BTreeMap<String, LocaleCoverage>,
}

/// The in-process view of coverage: any store that can be read can be measured,
/// with no state and no cache.
///
/// The live service measures coverage through maintained SQLite state instead
/// (`internals::coverage::CoverageReporter`). This trait is what read-only
/// stores - version snapshots - and the differential test against that state
/// use.
pub trait Coverage {
    fn coverage(&self, reference: &str) -> Result<CoverageReport>;
    /// One page of the reference keys `locale` does not translate, in ascending
    /// key order and resuming after `cursor`.
    fn missing_keys(
        &self,
        reference: &str,
        locale: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<String, String>>;
}

impl<T: KVRead + ?Sized> Coverage for T {
    fn coverage(&self, reference: &str) -> Result<CoverageReport> {
        coverage(self, reference)
    }

    fn missing_keys(
        &self,
        reference: &str,
        locale: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<String, String>> {
        missing_keys(self, reference, locale, cursor, limit)
    }
}

/// Measures how much of the `reference` locale every other locale in `store`
/// translates.
///
/// The reference is read once and every other locale is merge-joined against it
/// in one forward pass, so memory stays O(1) and each locale is read once.
pub fn coverage<S>(store: &S, reference: &str) -> Result<CoverageReport>
where
    S: KVRead + ?Sized,
{
    let locales = store.statistics()?.per_locale;
    require(&locales, reference)?;

    let reference_keys = collect_keys(store, reference)?;
    let mut covered = vec![false; reference_keys.len()];

    let mut per_locale: BTreeMap<String, LocaleCoverage> = BTreeMap::new();
    let mut present = 0usize;

    for locale in locales.keys().filter(|key| key.as_str() != reference) {
        covered.fill(false);
        let locale_present = mark_covered(store, locale, &reference_keys, &mut covered)?;
        present += locale_present;
        per_locale.insert(
            locale.clone(),
            LocaleCoverage {
                present: locale_present,
                missing: reference_keys.len() - locale_present,
                ratio: ratio(locale_present, reference_keys.len()),
            },
        );
    }

    Ok(CoverageReport {
        reference: reference.to_string(),
        reference_keys: reference_keys.len(),
        overall: ratio(present, reference_keys.len() * per_locale.len()),
        per_locale,
    })
}

/// One page of the reference keys `locale` does not translate.
///
/// A locale the store does not have translates nothing, so every reference key
/// is reported instead of an error: scouting a locale that is about to be
/// created is the point.
///
/// The cursor is the last key of the previous page; the scan resumes right
/// after it in the reference's key order, which is also the order the keys are
/// reported in.
pub fn missing_keys<S>(
    store: &S,
    reference: &str,
    locale: &str,
    cursor: Option<&str>,
    limit: usize,
) -> Result<Page<String, String>>
where
    S: KVRead + ?Sized,
{
    let locales = store.statistics()?.per_locale;
    require(&locales, reference)?;

    let mut entries = Entries::new_at(store, reference, cursor);
    let mut items: Vec<String> = Vec::new();
    let mut more = false;

    while let Some(entry) = entries.next_entry()? {
        if entry.value.is_empty() {
            continue;
        }
        if translates(store, locale, &entry.key)? {
            continue;
        }
        if items.len() == limit {
            more = true;
            break;
        }
        items.push(entry.key);
    }

    Ok(Page {
        next: if more { items.last().cloned() } else { None },
        items,
    })
}

fn require(per_locale: &BTreeMap<String, usize>, locale: &str) -> Result<()> {
    if per_locale.contains_key(locale) {
        return Ok(());
    }
    Err(StoreError::LocaleNotExist(locale.to_string()).into())
}

/// Flags the reference keys that `locale` translates, and returns how many
/// there are. Both sides are in ascending key order, so this is a merge join:
/// `index` only ever moves forward.
fn mark_covered<S>(
    store: &S,
    locale: &str,
    reference_keys: &[String],
    covered: &mut [bool],
) -> Result<usize>
where
    S: KVRead + ?Sized,
{
    let mut entries = Entries::new(store, locale);
    let mut index = 0usize;
    let mut present = 0usize;

    while let Some(entry) = entries.next_entry()? {
        while index < reference_keys.len() && reference_keys[index] < entry.key {
            index += 1;
        }
        if index == reference_keys.len() {
            // Every remaining reference key sorts before the rest of this locale.
            break;
        }
        if reference_keys[index] == entry.key {
            if !entry.value.is_empty() {
                covered[index] = true;
                present += 1;
            }
            index += 1;
        }
    }

    Ok(present)
}

/// The keys `locale` translates, in ascending order. Used on the reference
/// locale, where this is the key universe every other locale is measured over.
fn collect_keys<S>(store: &S, locale: &str) -> Result<Vec<String>>
where
    S: KVRead + ?Sized,
{
    let mut entries = Entries::new(store, locale);
    let mut keys = Vec::new();
    while let Some(entry) = entries.next_entry()? {
        if !entry.value.is_empty() {
            keys.push(entry.key);
        }
    }
    Ok(keys)
}

/// A key counts as translated when the locale holds a non-empty value for it.
/// A locale the store does not have is a legitimate 0%, not a failure; every
/// other store error is propagated.
fn translates<S>(store: &S, locale: &str, key: &str) -> Result<bool>
where
    S: KVRead + ?Sized,
{
    match store.get(locale, key) {
        Ok(Some(value)) => Ok(!value.is_empty()),
        Ok(None) => Ok(false),
        Err(err) => match err.downcast_ref::<StoreError>() {
            Some(StoreError::LocaleNotExist(_)) => Ok(false),
            _ => Err(err),
        },
    }
}

/// The entries of one locale in ascending key order, pulled one page at a time
/// so a scan never materialises the whole locale.
struct Entries<'s, S: KVRead + ?Sized> {
    store: &'s S,
    locale: &'s str,
    cursor: KVCursor,
    page: std::vec::IntoIter<KVEntry>,
    exhausted: bool,
}

impl<'s, S: KVRead + ?Sized> Entries<'s, S> {
    fn new(store: &'s S, locale: &'s str) -> Self {
        Self::new_at(store, locale, None)
    }

    /// Starts at the first key after `after`.
    fn new_at(store: &'s S, locale: &'s str, after: Option<&str>) -> Self {
        Entries {
            store,
            locale,
            cursor: KVCursor {
                order: SortOrder::Asc,
                filter: None,
                next: after.map(str::to_string),
            },
            page: Vec::new().into_iter(),
            exhausted: false,
        }
    }

    fn next_entry(&mut self) -> Result<Option<KVEntry>> {
        loop {
            if let Some(entry) = self.page.next() {
                return Ok(Some(entry));
            }
            if self.exhausted {
                return Ok(None);
            }
            let page = self
                .store
                .list(self.locale, &self.cursor, ENTRIES_PER_PAGE)?;
            self.exhausted = page.next.is_none();
            if let Some(next) = page.next {
                self.cursor = next;
            }
            self.page = page.items.into_iter();
        }
    }
}

fn ratio(present: usize, reference_keys: usize) -> f64 {
    if reference_keys == 0 {
        1.0
    } else {
        present as f64 / reference_keys as f64
    }
}
