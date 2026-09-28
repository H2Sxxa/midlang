use std::{error::Error, fmt::Display};

use anyhow::Result;
use serde::Serialize;

pub use crate::query::pagination::{QueryCursor, SortOrder};

pub mod redb;

/// Filter of a [`KVStore::list`] query: a case-insensitive substring matched
/// against the key or the value. redb only indexes the key order, so it is
/// applied while scanning.
pub type KVFilter = Option<String>;

/// Cursor of a [`KVStore::list`] query, whose position is the last key returned.
pub type KVCursor = QueryCursor<KVFilter, String>;

// "locale:namespace.key":"Value"
pub trait KVStore {
    fn set(&self, locale: &str, key: &str, value: &str) -> Result<ValueState>;
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>>;
    fn delete(&self, locale: &str, key: &str) -> Result<String>;
    fn delete_locale(&self, locale: &str) -> Result<()>;
    /// Returns one page of `limit` entries for `cursor`. Start with
    /// [`QueryCursor::new`] and pass the previous [`KVPage::next`] afterwards.
    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage>;
}

#[derive(Debug, Clone, Serialize)]
pub struct KVEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KVPage {
    pub entries: Vec<KVEntry>,
    /// `None` when this is the last page, otherwise the cursor for the next one.
    pub next: Option<KVCursor>,
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

pub use redb::RedbStore;
