use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Result;
use sqlx::{Pool, QueryBuilder, Sqlite, Transaction};

use crate::coverage::{CoverageReport, LocaleCoverage};
use crate::internals::worker::{WorkState, Workable};
use crate::query::Page;
use crate::store::{KVCursor, KVRead, SortOrder, StoreObserver};

const ENTRIES_PER_PAGE: usize = 1024;
const INSERT_BATCH: usize = 256;

enum Delta {
    Entry {
        locale: String,
        key: String,
        translated: bool,
    },
    Locale {
        locale: String,
    },
}

#[derive(Debug)]
pub enum CoverageError {
    Inconsistent {
        present: usize,
        reference_keys: usize,
    },
}

impl Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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

/// Maintains translation coverage in three normalized tables:
/// text is stored once in the key/locale dictionaries, while each translation
/// row contains only two integer IDs.
pub struct CoverageReporter {
    pool: Pool<Sqlite>,
    pending: Mutex<Vec<Delta>>,
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
        crate::internals::schema::ensure_tables(&self.pool).await?;
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS midlang_coverage_keys (
                id  INTEGER PRIMARY KEY,
                key TEXT NOT NULL UNIQUE
            )
            ",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS midlang_coverage_translations (
                key_id    INTEGER NOT NULL,
                locale_id INTEGER NOT NULL,
                PRIMARY KEY (key_id, locale_id)
            ) WITHOUT ROWID
            ",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "
            CREATE INDEX IF NOT EXISTS midlang_coverage_translations_locale_key
            ON midlang_coverage_translations (locale_id, key_id)
            ",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn resync<S>(&self, store: &S) -> Result<()>
    where
        S: KVRead + ?Sized,
    {
        self.flush().await?;
        let locales = store.statistics()?.per_locale;
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM midlang_coverage_translations")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM midlang_coverage_keys")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM midlang_locales")
            .execute(&mut *tx)
            .await?;

        for locale in locales.keys() {
            let locale_id = upsert_locale(&mut tx, locale).await?;
            let mut keys = Vec::new();
            let mut cursor = KVCursor::new(SortOrder::Asc, None);
            loop {
                let page = store.list(locale, &cursor, ENTRIES_PER_PAGE)?;
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
            insert_keys(&mut tx, locale_id, &keys).await?;
        }

        tx.commit().await?;
        self.generation.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

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

        if let Err(err) = self.apply_deltas(&deltas).await {
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
        let mut locales = HashMap::new();
        let mut keys = HashMap::new();

        for delta in deltas {
            match delta {
                Delta::Entry {
                    locale,
                    key,
                    translated: true,
                } => {
                    let locale_id = match locales.get(locale) {
                        Some(id) => *id,
                        None => {
                            let id = upsert_locale(&mut tx, locale).await?;
                            locales.insert(locale.clone(), id);
                            id
                        }
                    };
                    let key_id = match keys.get(key) {
                        Some(id) => *id,
                        None => {
                            let id = upsert_key(&mut tx, key).await?;
                            keys.insert(key.clone(), id);
                            id
                        }
                    };
                    sqlx::query(
                        "INSERT OR IGNORE INTO midlang_coverage_translations (key_id, locale_id)
                         VALUES (?, ?)",
                    )
                    .bind(key_id)
                    .bind(locale_id)
                    .execute(&mut *tx)
                    .await?;
                }
                Delta::Entry { locale, key, .. } => {
                    let locale_id = find_locale(&mut *tx, locale).await?;
                    let key_id = find_key(&mut *tx, key).await?;
                    if let (Some(locale_id), Some(key_id)) = (locale_id, key_id) {
                        sqlx::query(
                            "DELETE FROM midlang_coverage_translations
                             WHERE key_id = ? AND locale_id = ?",
                        )
                        .bind(key_id)
                        .bind(locale_id)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
                Delta::Locale { locale } => {
                    if let Some(locale_id) = find_locale(&mut *tx, locale).await? {
                        sqlx::query(
                            "DELETE FROM midlang_coverage_translations WHERE locale_id = ?",
                        )
                        .bind(locale_id)
                        .execute(&mut *tx)
                        .await?;
                        sqlx::query("DELETE FROM midlang_locales WHERE id = ?")
                            .bind(locale_id)
                            .execute(&mut *tx)
                            .await?;
                    }
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn report<S>(&self, store: &S, reference: &str) -> Result<Arc<CoverageReport>>
    where
        S: KVRead + ?Sized,
    {
        self.flush().await?;
        let generation = self.generation.load(Ordering::Acquire);
        if let Some((cached_generation, report)) = self
            .reports
            .read()
            .expect("coverage report cache poisoned")
            .get(reference)
            .cloned()
            && cached_generation == generation
        {
            return Ok(report);
        }

        let locales = store.statistics()?.per_locale;
        let reference_id = find_locale(&self.pool, reference)
            .await?
            .ok_or_else(|| crate::store::StoreError::LocaleNotExist(reference.to_string()))?;
        let reference_keys = self.count_keys(reference_id).await?;

        let mut per_locale = BTreeMap::new();
        let mut present_total = 0;
        for locale in locales.keys().filter(|locale| locale.as_str() != reference) {
            let present = match find_locale(&self.pool, locale).await? {
                Some(locale_id) => self.count_pair(reference_id, locale_id).await?,
                None => 0,
            };
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

    pub async fn missing_keys(
        &self,
        reference: &str,
        locale: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Page<String, String>> {
        self.flush().await?;
        let reference_id = find_locale(&self.pool, reference)
            .await?
            .ok_or_else(|| crate::store::StoreError::LocaleNotExist(reference.to_string()))?;
        let locale_id = find_locale(&self.pool, locale).await?;

        let mut rows = if let Some(locale_id) = locale_id {
            sqlx::query_scalar(
                "
                SELECT keys.key
                  FROM midlang_coverage_translations AS reference
                  JOIN midlang_coverage_keys AS keys ON keys.id = reference.key_id
                  LEFT JOIN midlang_coverage_translations AS translated
                    ON translated.key_id = reference.key_id AND translated.locale_id = ?
                 WHERE reference.locale_id = ?
                   AND translated.key_id IS NULL
                   AND (? IS NULL OR keys.key > ?)
                 ORDER BY keys.key
                 LIMIT ?
                ",
            )
            .bind(locale_id)
            .bind(reference_id)
            .bind(cursor)
            .bind(cursor)
            .bind(limit as i64 + 1)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_scalar(
                "
                SELECT keys.key
                  FROM midlang_coverage_translations AS reference
                  JOIN midlang_coverage_keys AS keys ON keys.id = reference.key_id
                 WHERE reference.locale_id = ?
                   AND (? IS NULL OR keys.key > ?)
                 ORDER BY keys.key
                 LIMIT ?
                ",
            )
            .bind(reference_id)
            .bind(cursor)
            .bind(cursor)
            .bind(limit as i64 + 1)
            .fetch_all(&self.pool)
            .await?
        };

        let next = if rows.len() > limit {
            rows.truncate(limit);
            rows.last().cloned()
        } else {
            None
        };
        Ok(Page { items: rows, next })
    }

    async fn count_keys(&self, locale_id: i64) -> Result<usize> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM midlang_coverage_translations WHERE locale_id = ?",
        )
        .bind(locale_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
    }

    async fn count_pair(&self, reference_id: i64, locale_id: i64) -> Result<usize> {
        let count: i64 = sqlx::query_scalar(
            "
            SELECT COUNT(*)
              FROM midlang_coverage_translations AS reference
              JOIN midlang_coverage_translations AS translated
                ON translated.key_id = reference.key_id
             WHERE reference.locale_id = ? AND translated.locale_id = ?
            ",
        )
        .bind(reference_id)
        .bind(locale_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count as usize)
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

async fn find_locale<'e, E>(executor: E, locale: &str) -> Result<Option<i64>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT id FROM midlang_locales WHERE locale = ?")
            .bind(locale)
            .fetch_optional(executor)
            .await?,
    )
}

async fn find_key<'e, E>(executor: E, key: &str) -> Result<Option<i64>>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_scalar("SELECT id FROM midlang_coverage_keys WHERE key = ?")
            .bind(key)
            .fetch_optional(executor)
            .await?,
    )
}

async fn upsert_locale(tx: &mut Transaction<'_, Sqlite>, locale: &str) -> Result<i64> {
    sqlx::query("INSERT OR IGNORE INTO midlang_locales (locale) VALUES (?)")
        .bind(locale)
        .execute(&mut **tx)
        .await?;
    Ok(find_locale(&mut **tx, locale)
        .await?
        .expect("locale inserted but not found"))
}

async fn upsert_key(tx: &mut Transaction<'_, Sqlite>, key: &str) -> Result<i64> {
    sqlx::query("INSERT OR IGNORE INTO midlang_coverage_keys (key) VALUES (?)")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(find_key(&mut **tx, key)
        .await?
        .expect("key inserted but not found"))
}

async fn insert_keys(
    tx: &mut Transaction<'_, Sqlite>,
    locale_id: i64,
    keys: &[String],
) -> Result<()> {
    let mut pairs = Vec::with_capacity(keys.len());
    for key in keys {
        pairs.push((upsert_key(tx, key).await?, locale_id));
    }
    for batch in pairs.chunks(INSERT_BATCH) {
        let mut builder = QueryBuilder::new(
            "INSERT OR IGNORE INTO midlang_coverage_translations (key_id, locale_id) ",
        );
        builder.push_values(batch, |mut row, (key_id, locale_id)| {
            row.push_bind(*key_id).push_bind(*locale_id);
        });
        builder.build().execute(&mut **tx).await?;
    }
    Ok(())
}

fn ratio(present: usize, reference_keys: usize) -> f64 {
    if reference_keys == 0 {
        1.0
    } else {
        present as f64 / reference_keys as f64
    }
}
