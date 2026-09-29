use std::sync::{Arc, RwLock};

use super::StoreObserver;

/// Shared observer registry used by cloneable stores.
#[derive(Clone, Default)]
pub struct ObserverRegistry {
    observers: Arc<RwLock<Vec<Arc<dyn StoreObserver>>>>,
}

impl ObserverRegistry {
    pub fn add(&self, observer: Arc<dyn StoreObserver>) {
        self.observers
            .write()
            .expect("store observer registry poisoned")
            .push(observer);
    }

    /// Replaces all observers. This preserves the legacy `attach_observer`
    /// semantics; use [`Self::add`] to append an observer.
    pub fn replace(&self, observer: Arc<dyn StoreObserver>) {
        let mut observers = self
            .observers
            .write()
            .expect("store observer registry poisoned");
        observers.clear();
        observers.push(observer);
    }

    pub fn notify_set(&self, locale: &str, key: &str, value: &str) {
        let observers = self.snapshot();
        for observer in observers {
            observer.on_set(locale, key, value);
        }
    }

    pub fn notify_delete(&self, locale: &str, key: &str, old_value: &str) {
        let observers = self.snapshot();
        for observer in observers {
            observer.on_delete(locale, key, old_value);
        }
    }

    pub fn notify_delete_locale(&self, locale: &str) {
        let observers = self.snapshot();
        for observer in observers {
            observer.on_delete_locale(locale);
        }
    }

    fn snapshot(&self) -> Vec<Arc<dyn StoreObserver>> {
        self.observers
            .read()
            .expect("store observer registry poisoned")
            .clone()
    }
}
