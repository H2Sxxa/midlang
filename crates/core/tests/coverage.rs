use midlang_core::{
    coverage::{coverage, missing_keys},
    store::{KVRead, KVStore, RedbStore, StoreError},
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn seeded_store(tag: &str, entries: &[(&str, &str, &str)]) -> (RedbStore, PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "midlang-coverage-{tag}-{}-{nanos}-{id}.rdb",
        std::process::id()
    ));

    let store = RedbStore::new(&path).unwrap();
    for (locale, key, value) in entries {
        store.set(locale, key, value).unwrap();
    }
    (store, path)
}

#[test]
fn complete_locales_report_no_missing_keys() {
    let (store, path) = seeded_store(
        "complete",
        &[
            ("en", "app.title", "MidLang"),
            ("en", "app.subtitle", "translations"),
            ("zh-cn", "app.title", "MidLang"),
            ("zh-cn", "app.subtitle", "翻译"),
        ],
    );

    let report = coverage(&store, "en").unwrap();

    assert_eq!(report.reference, "en");
    assert_eq!(report.reference_keys, 2);
    assert_eq!(report.overall, 1.0);
    // The reference locale is the universe, not one of the measured locales.
    assert_eq!(report.per_locale.len(), 1);

    let zh = &report.per_locale["zh-cn"];
    assert_eq!(zh.present, 2);
    assert_eq!(zh.missing, 0);
    assert_eq!(zh.ratio, 1.0);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn missing_keys_are_listed_per_locale_in_reference_order() {
    let (store, path) = seeded_store(
        "missing",
        &[
            ("en", "app.title", "MidLang"),
            ("en", "app.subtitle", "translations"),
            ("en", "app.footer", "footer"),
            ("de", "app.title", "MidLang"),
            // Present but empty, so it does not count as translated.
            ("de", "app.footer", ""),
            ("fr", "app.title", "MidLang"),
        ],
    );

    let report = coverage(&store, "en").unwrap();

    assert_eq!(report.reference_keys, 3);
    assert_eq!(report.per_locale.len(), 2);
    assert_eq!(report.overall, 1.0 / 3.0);

    for locale in ["de", "fr"] {
        let locale_coverage = &report.per_locale[locale];
        assert_eq!(locale_coverage.present, 1);
        assert_eq!(locale_coverage.missing, 2);
        assert_eq!(locale_coverage.ratio, 1.0 / 3.0);

        let listed = missing_keys(&store, "en", locale, None, 10).unwrap();
        assert_eq!(
            listed.items,
            vec!["app.footer".to_string(), "app.subtitle".to_string()]
        );
        assert_eq!(listed.next, None);
    }

    std::fs::remove_file(path).unwrap();
}

#[test]
fn a_locale_that_translates_nothing_is_at_zero() {
    let (store, path) = seeded_store(
        "new-locale",
        &[
            ("en", "app.title", "MidLang"),
            ("en", "app.subtitle", "translations"),
            // A key of its own does not count: coverage is measured over the
            // reference key set.
            ("fr", "page.other", "Autre"),
        ],
    );

    let report = coverage(&store, "en").unwrap();
    let fr = &report.per_locale["fr"];

    assert_eq!(fr.present, 0);
    assert_eq!(fr.missing, 2);
    assert_eq!(fr.ratio, 0.0);

    let listed = missing_keys(&store, "en", "fr", None, 10).unwrap();
    assert_eq!(
        listed.items,
        vec!["app.subtitle".to_string(), "app.title".to_string()]
    );
    assert_eq!(listed.next, None);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn an_unknown_reference_locale_is_an_error() {
    let (store, path) = seeded_store("unknown-reference", &[("de", "app.title", "MidLang")]);

    let error = coverage(&store, "en").unwrap_err();

    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::LocaleNotExist(locale)) if locale == "en"
    ));

    std::fs::remove_file(path).unwrap();
}

#[test]
fn any_read_only_store_can_be_measured() {
    // Both sides are `KVRead`, so coverage works on any store that only reads.
    let (store, path) = seeded_store(
        "trait-object",
        &[
            ("en", "app.title", "MidLang"),
            ("de", "app.title", "MidLang"),
        ],
    );
    let read_only: &dyn KVRead = &store;

    let report = coverage(read_only, "en").unwrap();

    assert_eq!(report.reference_keys, 1);
    assert_eq!(report.per_locale["de"].present, 1);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn missing_keys_pages_after_the_cursor() {
    let (store, path) = seeded_store(
        "paged-missing",
        &[
            ("en", "k1", "1"),
            ("en", "k2", "2"),
            ("en", "k3", "3"),
            ("en", "k4", "4"),
            ("de", "k1", "eins"),
        ],
    );

    let first = missing_keys(&store, "en", "de", None, 2).unwrap();
    assert_eq!(first.items, vec!["k2".to_string(), "k3".to_string()]);
    assert_eq!(first.next, Some("k3".to_string()));

    let second = missing_keys(&store, "en", "de", first.next.as_deref(), 2).unwrap();
    assert_eq!(second.items, vec!["k4".to_string()]);
    assert_eq!(second.next, None);

    std::fs::remove_file(path).unwrap();
}
