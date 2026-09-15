use std::{error::Error, fmt::Display};

use anyhow::Result;
pub mod redb;
// "locale:namespace.key":"Value"
pub trait KVStore {
    fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<ValueState>;
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>>;
    fn delete(&mut self, locale: &str, key: &str) -> Result<String>;
    fn delete_locale(&mut self, locale: &str) -> Result<()>;
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
