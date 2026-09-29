//! Cross-checks the maintained coverage state against the streaming report.
//!
//! `CoverageReporter` answers from SQLite state fed by store notifications,
//! while `coverage()` reads the store directly. Any sequence of writes has to
//! leave the two agreeing, so this drives deterministic pseudo-random writes
//! through the whole notification path and compares the reports field by field.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use midlang_core::{
    coverage::{coverage, missing_keys},
    internals::{InternalService, coverage::CoverageReporter},
    store::{KVRead, KVStore, RedbStore, StoreObserver},
};
use sqlx::sqlite::SqlitePoolOptions;

const REFERENCE: &str = "en";
const LOCALES: [&str; 4] = ["en", "zh-cn", "de", "fr"];
const KEYS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];
/// Locales the random sequence may drop wholesale. The reference defines the
/// key universe, so it has to survive every sequence.
const DROPPABLE: [&str; 3] = ["zh-cn", "de", "fr"];
const OPERATIONS: usize = 400;
const PAGE: usize = 2;
const MAX_PAGES: usize = 8;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Deterministic pseudo-random sequence, so a failing run replays exactly.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn pick<T: Copy>(&mut self, options: &[T]) -> T {
        options[(self.next() as usize) % options.len()]
    }
}

fn temp_store_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "midlang-coverage-reporter-{}-{nanos}-{id}.rdb",
        std::process::id()
    ));
    path
}

/// Applies one random write: a value, an empty value, a key deletion or a whole
/// locale deletion.
fn apply(lcg: &mut Lcg, store: &RedbStore) -> Result<()> {
    match lcg.next() % 10 {
        0..=4 => {
            let locale = lcg.pick(&LOCALES);
            let key = lcg.pick(&KEYS);
            store.set(locale, key, &format!("{locale}-{key}"))?;
        }
        5..=6 => {
            let locale = lcg.pick(&LOCALES);
            let key = lcg.pick(&KEYS);
            store.set(locale, key, "")?;
        }
        7..=8 => {
            let locale = lcg.pick(&LOCALES);
            let key = lcg.pick(&KEYS);
            // `get` on a locale the store does not have is an error, so the
            // existence check has to come first.
            if store.statistics()?.per_locale.contains_key(locale)
                && store.get(locale, key)?.is_some()
            {
                store.delete(locale, key)?;
            }
        }
        _ => {
            let locale = lcg.pick(&DROPPABLE);
            if store.statistics()?.per_locale.contains_key(locale) {
                store.delete_locale(locale)?;
            }
        }
    }
    Ok(())
}

/// Compares the maintained report and its paged missing-key listing against the
/// streaming oracle, field by field.
async fn assert_matches<S>(reporter: &CoverageReporter, store: &S) -> Result<()>
where
    S: KVRead + ?Sized,
{
    let streaming = coverage(store, REFERENCE)?;
    let maintained = reporter.report(store, REFERENCE).await?;
    assert_eq!(
        *maintained, streaming,
        "maintained report diverged from the store"
    );

    for locale in streaming.per_locale.keys() {
        let mut cursor: Option<String> = None;
        for page in 0..MAX_PAGES {
            let streamed = missing_keys(store, REFERENCE, locale, cursor.as_deref(), PAGE)?;
            let reported = reporter
                .missing_keys(REFERENCE, locale, cursor.as_deref(), PAGE)
                .await?;
            assert_eq!(
                reported.items, streamed.items,
                "missing keys diverged for {locale} on page {page}"
            );
            assert_eq!(
                reported.next, streamed.next,
                "missing key cursor diverged for {locale} on page {page}"
            );
            match streamed.next {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn reporter_matches_the_streaming_report_after_random_writes() -> Result<()> {
    for seed in [0x5EED_1234_ABCD_0001, 0x0BAD_C0DE_0000_0002] {
        run_scenario(seed).await?;
    }
    Ok(())
}

#[tokio::test]
async fn reporter_supports_more_than_63_locales() -> Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    let reporter = Arc::new(CoverageReporter::new(pool).await?);

    let mut internal = InternalService::default();
    internal.coverage = Some(reporter.clone());

    let store_path = temp_store_path();
    let store = RedbStore::new(&store_path)?;
    store.attach_observer(Arc::new(internal) as Arc<dyn StoreObserver>);

    store.set(REFERENCE, "shared", "source")?;
    for index in 0..70 {
        let locale = format!("locale-{index:02}");
        store.set(&locale, "shared", "translated")?;
    }

    let report = reporter.report(&store, REFERENCE).await?;
    assert_eq!(report.per_locale.len(), 70);
    assert!(
        report
            .per_locale
            .values()
            .all(|coverage| coverage.present == 1)
    );

    std::fs::remove_file(store_path)?;
    Ok(())
}

/// One store, one reporter and one deterministic write sequence.
async fn run_scenario(seed: u64) -> Result<()> {
    // One connection, so the in-memory database is shared by every query.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    let reporter = Arc::new(CoverageReporter::new(pool).await?);

    let mut internal = InternalService::default();
    internal.coverage = Some(reporter.clone());

    let store_path = temp_store_path();
    let store = RedbStore::new(&store_path)?;
    // The store talks to the internals service, which fans the write out to the
    // reporter, so the test covers the whole notification path.
    store.attach_observer(Arc::new(internal) as Arc<dyn StoreObserver>);

    for locale in LOCALES {
        for key in KEYS {
            store.set(locale, key, &format!("{locale}-{key}"))?;
        }
    }
    // A key only the other locale has must not enter the reference universe.
    store.set("fr", "only-fr", "unique")?;
    assert_matches(&reporter, &store).await?;

    let mut lcg = Lcg(seed);
    for operation in 0..OPERATIONS {
        apply(&mut lcg, &store)?;
        // Checking every step pins a divergence to the write that caused it
        // instead of letting a later write mask it.
        if let Err(err) = assert_matches(&reporter, &store).await {
            panic!("maintained state diverged after operation {operation}: {err}");
        }
    }

    // The startup rebuild has to land on the same state as the incremental path.
    reporter.resync(&store).await?;
    assert_matches(&reporter, &store).await?;

    std::fs::remove_file(store_path)?;
    Ok(())
}
