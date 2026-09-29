use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Result, anyhow, bail};
use sqlx::{Pool, QueryBuilder, Sqlite, Transaction};

use crate::coverage::{CoverageReport, LocaleCoverage};
use crate::internals::worker::{WorkState, Workable};
use crate::query::Page;
use crate::store::{KVCursor, KVRead, SortOrder, StoreObserver};

/// Entries read per page while priming from the store.
const ENTRIES_PER_PAGE: usize = 1024;
/// Rows per statement while priming; keeps the bind count well under SQLite's
/// variable limit.
const PRIME_BATCH: usize = 256;
/// One bit per locale lives in a signed 64-bit mask, and the sign bit is left
/// alone so masks stay positive.
const MAX_LOCALES: i64 = 63;

/// How one write changes the coverage state.
enum Delta {
    /// `translated` is the state after the write, not the value itself.
    Entry {
        locale: String,
        key: String,
        translated: bool,
    },
    /// Every key of the locale lost its translation.
    Locale { locale: String },
}

impl Delta {
    fn locale(&self) -> &str {
        match self {
            Delta::Entry { locale, .. } | Delta::Locale { locale } => locale,
        }
    }
}

#[derive(Debug)]
pub enum CoverageError {
    /// One bit per locale is kept in a signed 64-bit mask, so at most
    /// [`MAX_LOCALES`] locales can be tracked.
    TooManyLocales(i64),
    /// The maintained state disagrees with the store; the report would be
    /// silently wrong, so it is refused instead.
    Inconsistent {
        present: usize,
        reference_keys: usize,
    },
}

impl Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageError::TooManyLocales(id) => {
                write!(
                    f,
                    "coverage tracks at most {MAX_LOCALES} locales, got id {id}"
                )
            }
            CoverageError::Inconsistent {
                present,
                reference_keys,
            } => write!(
                f,
                "coverage state is inconsistent: {present} translated keys against {reference_keys} reference keys"
            ),
        }
    }
}

impl Error for CoverageError {}

/// Keeps per-key translation state in SQLite so that coverage can be answered
/// with indexed queries instead of a full scan, and so that the state survives
/// a restart.
///
/// One row per key holds a bitmask of the locales that translate it with a
/// non-empty value; the locale table maps a locale to its bit. Writes are
/// queued by the store's synchronous notifications and applied in one
/// transaction by [`CoverageReporter::flush`], which the workplace calls on its
/// tick and every read calls first, so a read never sees a half-applied batch.
pub struct CoverageReporter {
    pool: Pool<Sqlite>,
    pending: Mutex<Vec<Delta>>,
    /// Bumped whenever committed state changes; the report cache key.
    generation: AtomicU64,
    reports: RwLock<HashMap<String, (u64, Arc<CoverageReport>)>>,
}

impl CoverageReporter {
    pub async fn new(pool: Pool<Sqlite>) -> Result<Self> {
        let reporter = CoverageReporter {
            pool,
            pending: Mutex::new(Vec::new()),
            generation: AtomicU64::new(0),
            reports: RwLock::new(HashMap::new()),
        };
        reporter.ensure_tables().await?;
        Ok(reporter)
    }

    async fn ensure_tables(&self) -> Result<()> {
        sqlx::query(
            "
        CREATE TABLE IF NOT EXISTS midlang_coverage_locales (
            id     INTEGER PRIMARY KEY AUTOINCREMENT,
            locale TEXT NOT NULL UNIQUE
        )
        ",
        )
        .execute(&mut *self.pool.acquire().await?)
        .await?;

        sqlx::query(
            "
        CREATE TABLE IF NOT EXISTS midlang_coverage_keys (
            key  TEXT PRIMARY KEY NOT NULL,
            mask INTEGER NOT NULL
        ) WITHOUT ROWID
        ",
        )
        .execute(&mut *self.pool.acquire().await?)
        .await?;

        Ok(())
    }

    /// Rebuilds the whole table from `store`.
    ///
    /// Called at startup: the notifications only describe writes made while the
    /// process runs, so state left over from a previous run has to be replaced
    /// rather than trusted. Cost is one scan of the store.
    pub async fn resync<S>(&self, store: &S) -> Result<()>
    where
        S: KVRead + ?Sized,
    {
        self.flush().await?;
        let locales = store.statistics()?.per_locale;

        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM midlang_coverage_keys")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM midlang_coverage_locales")
            .execute(&mut *tx)
            .await?;

        for locale in locales.keys() {
            let id = upsert_locale(&mut tx, locale).await?;
            let bit = bit_of(id)?;
            let mut primed: Vec<(String, i64)> = Vec::new();
            let mut cursor = KVCursor::new(SortOrder::Asc, None);

            loop {
                let page = store.list(locale, &cursor, ENTRIES_PER_PAGE)?;
                for entry in page.items {
                    if !entry.value.is_empty() {
                        primed.push((entry.key, bit));
                    }
                }
                match page.next {
                    Some(next) => cursor = next,
                    None => break,
                }
            }

            insert_keys(&mut tx, &primed).await?;
        }
        tx.commit().await?;

        self.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    /// Applies every queued change in one transaction. Returns immediately when
    /// nothing is queued, which is what makes it safe to call before a read.
    pub async fn flush(&self) -> Result<()> {
        let deltas = {
            let mut pending = self
                .pending
                .lock()
                .expect("coverage pending queue poisoned");
            if pending.is_empty() {
                return Ok(());
            }
            std::mem::take(&mut *pending)
        };

        let result = self.apply_deltas(&deltas).await;
        if let Err(err) = result {
            let mut pending = self
                .pending
                .lock()
                .expect("coverage pending queue poisoned");
            let mut retry = deltas;
            retry.append(&mut *pending);
            *pending = retry;
            return Err(err);
        }

        Ok(())
    }

    async fn apply_deltas(&self, deltas: &[Delta]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        let mut ids: HashMap<String, i64> = HashMap::new();
        for locale in distinct_locales(deltas) {
            ids.insert(locale.to_string(), upsert_locale(&mut tx, locale).await?);
        }

        for delta in deltas {
            let bit = bit_of(locale_id(&ids, delta.locale())?)?;
            match delta {
                Delta::Entry {
                    key, translated, ..
                } if *translated => {
                    insert_keys(&mut tx, &[(key.clone(), bit)]).await?;
                }
                Delta::Entry { key, .. } => {
                    sqlx::query("UPDATE midlang_coverage_keys SET mask = mask & ? WHERE key = ?")
                        .bind(!bit)
                        .bind(key)
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("DELETE FROM midlang_coverage_keys WHERE key = ? AND mask = 0")
                        .bind(key)
                        .execute(&mut *tx)
                        .await?;
                }
                Delta::Locale { .. } => {
                    sqlx::query(
                        "UPDATE midlang_coverage_keys SET mask = mask & ? WHERE (mask & ?) != 0",
                    )
                    .bind(!bit)
                    .bind(bit)
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query("DELETE FROM midlang_coverage_keys WHERE mask = 0")
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
        for locale in distinct_locales(deltas) {
            if last_delta_is_locale_delete(deltas, locale) {
                sqlx::query("DELETE FROM midlang_coverage_locales WHERE locale = ?")
                    .bind(locale)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        tx.commit().await?;

        Ok(())
    }

    /// Coverage of `reference` for every locale the store has.
    ///
    /// Flushes first, so the answer includes writes that have not reached the
    /// workplace tick yet.
    pub async fn report<S>(&self, store: &S, reference: &str) -> Result<Arc<CoverageReport>>
    where
        S: KVRead + ?Sized,
    {
        self.flush().await?;
        let generation = self.generation.load(Ordering::Acquire);
        let cached = self
            .reports
            .read()
            .expect("coverage report cache poisoned")
            .get(reference)
            .cloned();
        if let Some((cached_generation, report)) = cached
            && cached_generation == generation
        {
            return Ok(report);
        }

        let locales = store.statistics()?.per_locale;
        if !locales.contains_key(reference) {
            return Err(crate::store::StoreError::LocaleNotExist(reference.to_string()).into());
        }

        let reference_bit = self.bit_of_locale(reference).await?.unwrap_or(0);
        let reference_keys = self.count_keys(reference_bit).await?;

        let mut per_locale: BTreeMap<String, LocaleCoverage> = BTreeMap::new();
        let mut present_total = 0usize;
        for locale in locales.keys().filter(|locale| locale.as_str() != reference) {
            let bit = self.bit_of_locale(locale).await?.unwrap_or(0);
            // A key is translated by both locales when its mask holds the union
            // of their bits; intersecting the bits themselves is always empty.
            let present = self.count_keys(reference_bit | bit).await?;
            let missing =
                reference_keys
                    .checked_sub(present)
                    .ok_or(CoverageError::Inconsistent {
                        present,
                        reference_keys,
                    })?;

            present_total += present;
            per_locale.insert(
                locale.clone(),
                LocaleCoverage {
                    present,
                    missing,
                    ratio: ratio(present, reference_keys),
                },
            );
        }

        let report = Arc::new(CoverageReport {
            reference: reference.to_string(),
            reference_keys,
            overall: ratio(present_total, reference_keys * per_locale.len()),
            per_locale,
        });
        self.reports
            .write()
            .expect("coverage report cache poisoned")
            .insert(reference.to_string(), (generation, report.clone()));

        Ok(report)
    }

    /// One page of the reference keys `locale` does not translate, in ascending
    /// key order and resuming after `cursor`.
    pub async fn missing_keys(
        &self,
        reference: &str,
        locale: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<String, String>> {
        self.flush().await?;
        let reference_bit = self
            .bit_of_locale(reference)
            .await?
            .ok_or_else(|| crate::store::StoreError::LocaleNotExist(reference.to_string()))?;
        let locale_bit = self.bit_of_locale(locale).await?.unwrap_or(0);

        // One row past the page tells whether a next page exists.
        let mut rows: Vec<String> = sqlx::query_scalar(
            "
            SELECT key FROM midlang_coverage_keys
             WHERE (mask & ?) != 0
               AND (mask & ?) = 0
               AND (? IS NULL OR key > ?)
             ORDER BY key
             LIMIT ?
            ",
        )
        .bind(reference_bit)
        .bind(locale_bit)
        .bind(cursor)
        .bind(cursor)
        .bind(limit as i64 + 1)
        .fetch_all(&self.pool)
        .await?;

        let next = if rows.len() > limit {
            rows.truncate(limit);
            rows.last().cloned()
        } else {
            None
        };

        Ok(Page { items: rows, next })
    }

    async fn bit_of_locale(&self, locale: &str) -> Result<Option<i64>> {
        let id: Option<i64> =
            sqlx::query_scalar("SELECT id FROM midlang_coverage_locales WHERE locale = ?")
                .bind(locale)
                .fetch_optional(&self.pool)
                .await?;
        match id {
            Some(id) => Ok(Some(bit_of(id)?)),
            None => Ok(None),
        }
    }

    /// Rows whose mask contains every bit of `mask`. `mask == 0` counts nothing
    /// rather than everything, so an unknown locale measures 0%.
    async fn count_keys(&self, mask: i64) -> Result<usize> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM midlang_coverage_keys WHERE (? != 0 AND (mask & ?) = ?)",
        )
        .bind(mask)
        .bind(mask)
        .bind(mask)
        .fetch_one(&self.pool)
        .await?;
        usize::try_from(count).map_err(|_| anyhow!("coverage counted {count} keys"))
    }

    fn queue(&self, delta: Delta) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.pending
            .lock()
            .expect("coverage pending queue poisoned")
            .push(delta);
    }
}

impl StoreObserver for CoverageReporter {
    fn on_set(&self, locale: &str, key: &str, value: &str) {
        self.queue(Delta::Entry {
            locale: locale.to_string(),
            key: key.to_string(),
            translated: !value.is_empty(),
        });
    }

    fn on_delete(&self, locale: &str, key: &str, old_value: &str) {
        // A key that was never translated has nothing to clear.
        if old_value.is_empty() {
            return;
        }
        self.queue(Delta::Entry {
            locale: locale.to_string(),
            key: key.to_string(),
            translated: false,
        });
    }

    fn on_delete_locale(&self, locale: &str) {
        self.queue(Delta::Locale {
            locale: locale.to_string(),
        });
    }
}

#[async_trait::async_trait]
impl Workable for CoverageReporter {
    async fn work(&self) -> Result<WorkState> {
        self.flush().await?;
        Ok(WorkState::ACTIVE)
    }
}

fn distinct_locales(deltas: &[Delta]) -> Vec<&str> {
    let mut locales: Vec<&str> = deltas.iter().map(Delta::locale).collect();
    locales.sort_unstable();
    locales.dedup();
    locales
}

fn locale_id(ids: &HashMap<String, i64>, locale: &str) -> Result<i64> {
    ids.get(locale)
        .copied()
        .ok_or_else(|| anyhow!("coverage lost the id of locale '{locale}'"))
}

async fn upsert_locale(tx: &mut Transaction<'_, Sqlite>, locale: &str) -> Result<i64> {
    let existing: Option<i64> =
        sqlx::query_scalar("SELECT id FROM midlang_coverage_locales WHERE locale = ?")
            .bind(locale)
            .fetch_optional(&mut **tx)
            .await?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let used: Vec<i64> = sqlx::query_scalar("SELECT id FROM midlang_coverage_locales")
        .fetch_all(&mut **tx)
        .await?;
    let id = (1..=MAX_LOCALES)
        .find(|id| !used.contains(id))
        .ok_or(CoverageError::TooManyLocales(MAX_LOCALES + 1))?;
    sqlx::query("INSERT INTO midlang_coverage_locales (id, locale) VALUES (?, ?)")
        .bind(id)
        .bind(locale)
        .execute(&mut **tx)
        .await?;
    Ok(id)
}

fn last_delta_is_locale_delete(deltas: &[Delta], locale: &str) -> bool {
    deltas
        .iter()
        .rev()
        .find(|delta| delta.locale() == locale)
        .is_some_and(|delta| matches!(delta, Delta::Locale { .. }))
}

/// Sets the given bits, or-ing them into what a key already has.
async fn insert_keys(tx: &mut Transaction<'_, Sqlite>, keys: &[(String, i64)]) -> Result<()> {
    for batch in keys.chunks(PRIME_BATCH) {
        let mut builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT INTO midlang_coverage_keys (key, mask) ");
        builder.push_values(batch, |mut row, (key, bit)| {
            row.push_bind(key.as_str()).push_bind(*bit);
        });
        builder.push(" ON CONFLICT (key) DO UPDATE SET mask = mask | excluded.mask");
        builder.build().execute(&mut **tx).await?;
    }
    Ok(())
}

fn bit_of(id: i64) -> Result<i64> {
    if !(1..=MAX_LOCALES).contains(&id) {
        bail!(CoverageError::TooManyLocales(id));
    }
    Ok(1i64 << (id - 1))
}

fn ratio(present: usize, reference_keys: usize) -> f64 {
    if reference_keys == 0 {
        1.0
    } else {
        present as f64 / reference_keys as f64
    }
}
