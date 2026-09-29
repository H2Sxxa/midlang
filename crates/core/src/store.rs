use std::{collections::BTreeMap, error::Error, fmt::Display, sync::Arc};

use anyhow::Result;
use serde::Serialize;

pub use crate::query::{Page, QueryCursor, SortOrder};
pub mod mem;
pub mod observer;
pub mod redb;
pub use observer::ObserverRegistry;

// "locale:namespace.key":"Value"

/// Filter of a [`KVRead::list`] query: a case-insensitive substring matched
/// against the key or the value. redb only indexes the key order, so it is
/// applied while scanning.
pub type KVFilter = Option<String>;

/// Cursor of a [`KVRead::list`] query, whose position is the last key returned.
pub type KVCursor = QueryCursor<KVFilter, String>;

/// Read half of the store: implemented by the live store and, once versioned
/// reading lands, by read-only snapshots of it.
pub trait KVRead {
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>>;
    /// Returns one page of `limit` entries for `cursor`. Start with
    /// [`QueryCursor::new`] and pass the previous [`Page::next`] afterwards.
    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage>;
    /// Locale and entry counts read from table metadata, without scanning rows.
    fn statistics(&self) -> Result<KVStatistics>;
}

/// Write half of the store. Only the live store implements it, so a version
/// snapshot has no write methods to call in the first place.
pub trait KVStore: KVRead + Send + Sync {
    /// Adds an observer without removing existing observers.
    fn add_observer(&self, observer: Arc<dyn StoreObserver>);
    fn set(&self, locale: &str, key: &str, value: &str) -> Result<ValueState>;
    fn delete(&self, locale: &str, key: &str) -> Result<String>;
    fn delete_locale(&self, locale: &str) -> Result<()>;
}

impl<T: KVRead + ?Sized> KVRead for Arc<T> {
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>> {
        (**self).get(locale, key)
    }

    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage> {
        (**self).list(locale, cursor, limit)
    }

    fn statistics(&self) -> Result<KVStatistics> {
        (**self).statistics()
    }
}

impl<T: KVStore + ?Sized> KVStore for Arc<T> {
    fn add_observer(&self, observer: Arc<dyn StoreObserver>) {
        (**self).add_observer(observer)
    }

    fn set(&self, locale: &str, key: &str, value: &str) -> Result<ValueState> {
        (**self).set(locale, key, value)
    }

    fn delete(&self, locale: &str, key: &str) -> Result<String> {
        (**self).delete(locale, key)
    }

    fn delete_locale(&self, locale: &str) -> Result<()> {
        (**self).delete_locale(locale)
    }
}

/// Write events a live store publishes so that services can keep derived state
/// in step without the store knowing about them.
///
/// Implementations are called from the store's synchronous write path, after
/// the write has committed. They must therefore not block, must not fail and
/// must not write back into the store; a service that needs to do asynchronous
/// work (SQLite, HTTP) has to queue the event and flush it elsewhere. A store
/// without observers snapshots an empty registry per write.
pub trait StoreObserver: Send + Sync {
    fn on_set(&self, locale: &str, key: &str, value: &str);
    fn on_delete(&self, locale: &str, key: &str, old_value: &str);
    fn on_delete_locale(&self, locale: &str);
}

#[derive(Debug, Clone, Serialize)]
pub struct KVEntry {
    pub key: String,
    pub value: String,
}

/// One page of entries plus the cursor that resumes the listing.
pub type KVPage = Page<KVEntry, KVCursor>;

#[derive(Debug, Clone, Serialize)]
pub struct KVStatistics {
    pub locales: usize,
    pub entries: usize,
    pub per_locale: BTreeMap<String, usize>,
}

#[derive(Debug)]
pub enum StoreError {
    // locale not exist
    LocaleNotExist(String),
    // key not exist
    LocaleKeyNotExist(String, String),
}

impl Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::LocaleNotExist(locale) => {
                write!(f, "Locale '{}' does not exist", locale)
            }
            StoreError::LocaleKeyNotExist(locale, key) => {
                write!(f, "Key '{}' in locale '{}' does not exist", key, locale)
            }
        }
    }
}
impl Error for StoreError {}

#[derive(Debug)]
pub enum ValueState {
    Created,
    // Old Value
    Updated(String),
    Deleted(String),
}

pub use mem::MemStore;
pub use redb::RedbStore;
