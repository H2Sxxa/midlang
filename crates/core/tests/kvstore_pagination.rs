use midlang_core::store::{KVCursor, KVEntry, KVRead, KVStore, RedbStore, SortOrder, StoreError};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SEEDED: usize = 25;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

fn temp_db_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "midlang-kv-pagination-{}-{}-{}.rdb",
        std::process::id(),
        nanos,
        id
    ));
    path
}

fn seeded_store() -> (RedbStore, PathBuf) {
    let path = temp_db_path();
    let store = RedbStore::new(&path).unwrap();
    for i in 0..SEEDED {
        store
            .set("en", &format!("k{i:02}"), &format!("val-{i:02}"))
            .unwrap();
    }
    (store, path)
}

fn list_all(
    store: &RedbStore,
    keyword: Option<&str>,
    limit: usize,
    order: SortOrder,
) -> Vec<KVEntry> {
    let mut cursor = KVCursor::new(order, keyword.map(str::to_string));
    let mut all: Vec<KVEntry> = Vec::new();
    for _ in 0..SEEDED + 1 {
        let page = store.list("en", &cursor, limit).unwrap();
        assert!(page.items.len() <= limit);
        all.extend(page.items);
        match page.next {
            Some(next) => cursor = next,
            None => return all,
        }
    }
    panic!("pagination did not terminate");
}

#[test]
fn asc_pagination_covers_every_entry_in_order() {
    let (store, path) = seeded_store();

    let entries = list_all(&store, None, 10, SortOrder::Asc);
    let keys: Vec<String> = entries.iter().map(|entry| entry.key.clone()).collect();
    let expected: Vec<String> = (0..SEEDED).map(|i| format!("k{i:02}")).collect();
    assert_eq!(keys, expected);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn desc_pagination_is_the_reverse_of_asc() {
    let (store, path) = seeded_store();

    let entries = list_all(&store, None, 7, SortOrder::Desc);
    let keys: Vec<String> = entries.iter().map(|entry| entry.key.clone()).collect();
    let mut expected: Vec<String> = (0..SEEDED).map(|i| format!("k{i:02}")).collect();
    expected.reverse();
    assert_eq!(keys, expected);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn keyword_is_a_substring_match() {
    let (store, path) = seeded_store();
    store.set("en", "a.b.c", "x").unwrap();
    store.set("en", "checkout.button.label", "y").unwrap();

    // Substring, not prefix: "b.c" still matches "a.b.c".
    let entries = list_all(&store, Some("b.c"), 10, SortOrder::Asc);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "a.b.c");

    // Case-insensitive, matching both keys and values.
    let by_key = list_all(&store, Some("CHECKOUT"), 10, SortOrder::Asc);
    assert_eq!(by_key.len(), 1);
    assert_eq!(by_key[0].key, "checkout.button.label");

    let by_value = list_all(&store, Some("VAL-2"), 100, SortOrder::Asc);
    assert_eq!(by_value.len(), 5);

    assert!(list_all(&store, Some("zzz"), 10, SortOrder::Asc).is_empty());

    std::fs::remove_file(path).unwrap();
}

#[test]
fn cursor_round_trips_through_a_token() {
    let mut cursor = KVCursor::new(SortOrder::Desc, Some("k1".to_string()));
    cursor.next = Some("k17".to_string());

    let decoded = KVCursor::decode(&cursor.encode().unwrap()).unwrap();
    assert_eq!(decoded.order, SortOrder::Desc);
    assert_eq!(decoded.filter.as_deref(), Some("k1"));
    assert_eq!(decoded.next.as_deref(), Some("k17"));
}

#[test]
fn statistics_counts_entries_per_locale() {
    let (store, path) = seeded_store();

    let stats = store.statistics().unwrap();
    assert_eq!(stats.locales, 1);
    assert_eq!(stats.entries, SEEDED);
    assert_eq!(stats.per_locale.get("en"), Some(&SEEDED));

    store.set("de", "hallo", "Hallo").unwrap();
    let stats = store.statistics().unwrap();
    assert_eq!(stats.locales, 2);
    assert_eq!(stats.entries, SEEDED + 1);
    assert_eq!(stats.per_locale.get("de"), Some(&1));

    std::fs::remove_file(path).unwrap();
}

#[test]
fn missing_locale_is_reported() {
    let (store, path) = seeded_store();

    let error = store
        .list("de", &KVCursor::new(SortOrder::Asc, None), 5)
        .unwrap_err();
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::LocaleNotExist(_))
    ));

    std::fs::remove_file(path).unwrap();
}
