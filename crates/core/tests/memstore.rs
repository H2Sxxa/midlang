use midlang_core::store::{KVCursor, KVRead, KVStore, MemStore, SortOrder, StoreError, ValueState};

fn list_all(store: &MemStore, locale: &str, limit: usize, order: SortOrder) -> Vec<String> {
    let mut cursor = KVCursor::new(order, None);
    let mut keys = Vec::new();
    loop {
        let page = store.list(locale, &cursor, limit).unwrap();
        keys.extend(page.items.into_iter().map(|entry| entry.key));
        match page.next {
            Some(next) => cursor = next,
            None => return keys,
        }
    }
}

#[test]
fn supports_writes_reads_and_updates() {
    let store = MemStore::new();
    assert!(matches!(
        store.set("en", "title", "Hello").unwrap(),
        ValueState::Created
    ));
    assert_eq!(store.get("en", "title").unwrap().as_deref(), Some("Hello"));
    assert!(matches!(
        store.set("en", "title", "Hi").unwrap(),
        ValueState::Updated(old) if old == "Hello"
    ));
}

#[test]
fn paginates_both_directions_and_filters_values() {
    let store = MemStore::default();
    for (key, value) in [("a", "one"), ("b", "Two"), ("c", "three"), ("d", "four")] {
        store.set("en", key, value).unwrap();
    }

    assert_eq!(
        list_all(&store, "en", 2, SortOrder::Asc),
        vec!["a", "b", "c", "d"]
    );
    assert_eq!(
        list_all(&store, "en", 2, SortOrder::Desc),
        vec!["d", "c", "b", "a"]
    );

    let page = store
        .list("en", &KVCursor::new(SortOrder::Asc, Some("TWO".into())), 10)
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].key, "b");
}

#[test]
fn preserves_empty_locales_and_avoids_key_collisions() {
    let store = MemStore::new();
    store.set("a", "b.c", "first").unwrap();
    store.set("a.b", "c", "second").unwrap();
    assert_eq!(store.get("a", "b.c").unwrap().as_deref(), Some("first"));
    assert_eq!(store.get("a.b", "c").unwrap().as_deref(), Some("second"));

    assert_eq!(store.delete("a", "b.c").unwrap(), "first");
    assert!(store.get("a", "b.c").unwrap().is_none());
    let stats = store.statistics().unwrap();
    assert_eq!(stats.per_locale.get("a"), Some(&0));
    assert_eq!(stats.per_locale.get("a.b"), Some(&1));

    store.delete_locale("a").unwrap();
    assert!(matches!(
        store.get("a", "anything").unwrap_err().downcast_ref::<StoreError>(),
        Some(StoreError::LocaleNotExist(locale)) if locale == "a"
    ));
}

#[test]
fn reports_missing_locale_and_key() {
    let store = MemStore::new();
    let error = store.get("en", "missing").unwrap_err();
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::LocaleNotExist(locale)) if locale == "en"
    ));

    store.set("en", "present", "value").unwrap();
    let error = store.delete("en", "missing").unwrap_err();
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::LocaleKeyNotExist(locale, key)) if locale == "en" && key == "missing"
    ));
}
