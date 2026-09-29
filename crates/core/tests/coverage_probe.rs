//! Measures the coverage join against a hash-index variant of the same report.
//!
//! Both run against an in-memory store, so the numbers are the cost of the
//! comparison itself and not of redb I/O.

use anyhow::Result;
use midlang_core::{
    coverage::{CoverageReport, LocaleCoverage, coverage},
    store::{KVCursor, KVEntry, KVPage, KVRead, KVStatistics, SortOrder, StoreError},
};
use std::collections::{BTreeMap, HashMap};
use std::ops::Bound;
use std::time::{Duration, Instant};

const REFERENCE: &str = "en";
const REFERENCE_KEYS: usize = 100_000;
const OTHER_LOCALES: usize = 7;
/// Every `MISSING_EVERY`-th key is absent from the other locales.
const MISSING_EVERY: usize = 10;
const PAGES: usize = 1024;
const REPS: usize = 3;

struct MemStore {
    locales: BTreeMap<String, BTreeMap<String, String>>,
}

impl MemStore {
    fn seeded() -> Self {
        let reference: BTreeMap<String, String> = (0..REFERENCE_KEYS)
            .map(|i| (format!("app.namespace.key{i:06}"), format!("value {i}")))
            .collect();

        let mut locales = BTreeMap::new();
        locales.insert(REFERENCE.to_string(), reference.clone());
        for index in 0..OTHER_LOCALES {
            let table = reference
                .iter()
                .enumerate()
                .filter(|(position, _)| position % MISSING_EVERY != 0)
                .map(|(_, (key, value))| (key.clone(), value.clone()))
                .collect();
            locales.insert(format!("l{index:02}"), table);
        }

        MemStore { locales }
    }
}

impl KVRead for MemStore {
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>> {
        match self.locales.get(locale) {
            Some(table) => Ok(table.get(key).cloned()),
            None => Err(StoreError::LocaleNotExist(locale.to_string()).into()),
        }
    }

    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage> {
        let Some(table) = self.locales.get(locale) else {
            return Err(StoreError::LocaleNotExist(locale.to_string()).into());
        };
        let start = match cursor.next.as_ref() {
            Some(key) => Bound::Excluded(key.clone()),
            None => Bound::Unbounded,
        };

        let mut items: Vec<KVEntry> = Vec::new();
        let mut more = false;
        for (key, value) in table.range((start, Bound::Unbounded)) {
            if items.len() == limit {
                more = true;
                break;
            }
            items.push(KVEntry {
                key: key.clone(),
                value: value.clone(),
            });
        }

        Ok(KVPage {
            next: if more {
                items.last().map(|entry| KVCursor {
                    order: SortOrder::Asc,
                    filter: None,
                    next: Some(entry.key.clone()),
                })
            } else {
                None
            },
            items,
        })
    }

    fn statistics(&self) -> Result<KVStatistics> {
        Ok(KVStatistics {
            locales: self.locales.len(),
            entries: self.locales.values().map(BTreeMap::len).sum(),
            per_locale: self
                .locales
                .iter()
                .map(|(locale, table)| (locale.clone(), table.len()))
                .collect(),
        })
    }
}

/// The same report with a key-to-position index instead of an ordered walk.
fn hash_coverage<S>(store: &S, reference: &str) -> Result<CoverageReport>
where
    S: KVRead + ?Sized,
{
    let locales = store.statistics()?.per_locale;
    let reference_keys = collect_keys(store, reference)?;
    let index_of: HashMap<&str, usize> = reference_keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), index))
        .collect();

    let mut covered = vec![false; reference_keys.len()];
    let mut per_locale: BTreeMap<String, LocaleCoverage> = BTreeMap::new();
    let mut present = 0usize;

    for locale in locales.keys().filter(|key| key.as_str() != reference) {
        covered.fill(false);
        let mut locale_present = 0usize;
        let mut cursor = KVCursor::new(SortOrder::Asc, None);
        loop {
            let page = store.list(locale, &cursor, PAGES)?;
            for entry in page.items {
                let Some(index) = index_of.get(entry.key.as_str()) else {
                    continue;
                };
                if !entry.value.is_empty() && !covered[*index] {
                    covered[*index] = true;
                    locale_present += 1;
                }
            }
            match page.next {
                Some(next) => cursor = next,
                None => break,
            }
        }

        let missing = covered.iter().filter(|covered| !**covered).count();
        present += locale_present;
        per_locale.insert(
            locale.clone(),
            LocaleCoverage {
                present: locale_present,
                missing,
                ratio: locale_present as f64 / reference_keys.len() as f64,
            },
        );
    }

    Ok(CoverageReport {
        reference: reference.to_string(),
        reference_keys: reference_keys.len(),
        overall: present as f64 / (reference_keys.len() * per_locale.len()) as f64,
        per_locale,
    })
}

fn collect_keys<S>(store: &S, locale: &str) -> Result<Vec<String>>
where
    S: KVRead + ?Sized,
{
    let mut keys = Vec::new();
    let mut cursor = KVCursor::new(SortOrder::Asc, None);
    loop {
        let page = store.list(locale, &cursor, PAGES)?;
        keys.extend(
            page.items
                .into_iter()
                .filter(|entry| !entry.value.is_empty())
                .map(|entry| entry.key),
        );
        match page.next {
            Some(next) => cursor = next,
            None => break,
        }
    }
    Ok(keys)
}

fn timed(label: &str, run: impl Fn() -> CoverageReport) -> (CoverageReport, Duration) {
    let start = Instant::now();
    let report = run();
    let elapsed = start.elapsed();
    println!(
        "  {label:<12} {:?}  ({} locales, overall {:.4})",
        elapsed,
        report.per_locale.len(),
        report.overall
    );
    (report, elapsed)
}

#[test]
fn probe_merge_join_against_hash_index() {
    let build = Instant::now();
    let store = MemStore::seeded();
    println!(
        "seeded {} locales x {} keys in {:?}",
        OTHER_LOCALES + 1,
        REFERENCE_KEYS,
        build.elapsed()
    );

    let (merge, _) = timed("merge join", || coverage(&store, REFERENCE).unwrap());
    let (hashed, _) = timed("hash index", || hash_coverage(&store, REFERENCE).unwrap());

    assert_eq!(merge.reference_keys, hashed.reference_keys);
    assert_eq!(merge, hashed, "the two variants disagree");

    let mut merge_total = Duration::ZERO;
    let mut hash_total = Duration::ZERO;
    for _ in 0..REPS {
        let (_, elapsed) = timed("merge join", || coverage(&store, REFERENCE).unwrap());
        merge_total += elapsed;
        let (_, elapsed) = timed("hash index", || hash_coverage(&store, REFERENCE).unwrap());
        hash_total += elapsed;
    }
    println!(
        "over {REPS} runs: merge {:?}, hash {:?}",
        merge_total / REPS as u32,
        hash_total / REPS as u32
    );
}
