use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use midlang_core::store::{KVStore, MemStore, ObserverRegistry, StoreObserver};

struct CountingObserver {
    sets: AtomicUsize,
    deletes: AtomicUsize,
    locale_deletes: AtomicUsize,
}

impl CountingObserver {
    fn new() -> Self {
        Self {
            sets: AtomicUsize::new(0),
            deletes: AtomicUsize::new(0),
            locale_deletes: AtomicUsize::new(0),
        }
    }
}

impl StoreObserver for CountingObserver {
    fn on_set(&self, _locale: &str, _key: &str, _value: &str) {
        self.sets.fetch_add(1, Ordering::Relaxed);
    }

    fn on_delete(&self, _locale: &str, _key: &str, _old_value: &str) {
        self.deletes.fetch_add(1, Ordering::Relaxed);
    }

    fn on_delete_locale(&self, _locale: &str) {
        self.locale_deletes.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn cloned_stores_share_multiple_observers() {
    let store = MemStore::new();
    let cloned = store.clone();
    let first = Arc::new(CountingObserver::new());
    let second = Arc::new(CountingObserver::new());

    store.add_observer(first.clone());
    cloned.add_observer(second.clone());

    cloned.set("en", "hello", "Hello").unwrap();

    for observer in [&first, &second] {
        assert_eq!(observer.sets.load(Ordering::Relaxed), 1);
        assert_eq!(observer.deletes.load(Ordering::Relaxed), 0);
        assert_eq!(observer.locale_deletes.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn multiple_observers_receive_all_event_types() {
    let store = MemStore::new();
    let first = Arc::new(CountingObserver::new());
    let second = Arc::new(CountingObserver::new());

    store.add_observer(first.clone());
    store.add_observer(second.clone());
    store.set("en", "hello", "Hello").unwrap();
    store.delete("en", "hello").unwrap();
    store.set("en", "bye", "Bye").unwrap();
    store.delete_locale("en").unwrap();

    let registry = ObserverRegistry::default();
    registry.add(first.clone());
    registry.add(second.clone());
    registry.notify_delete_locale("fr");

    for observer in [&first, &second] {
        assert_eq!(observer.sets.load(Ordering::Relaxed), 2);
        assert_eq!(observer.deletes.load(Ordering::Relaxed), 1);
        assert_eq!(observer.locale_deletes.load(Ordering::Relaxed), 2);
    }
}

#[test]
fn attach_observer_replaces_existing_observers() {
    let store = MemStore::new();
    let first = Arc::new(CountingObserver::new());
    let second = Arc::new(CountingObserver::new());

    store.add_observer(first.clone());
    store.attach_observer(second.clone());
    store.set("en", "hello", "Hello").unwrap();

    assert_eq!(first.sets.load(Ordering::Relaxed), 0);
    assert_eq!(second.sets.load(Ordering::Relaxed), 1);
}
