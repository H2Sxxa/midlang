use anyhow::Result;
pub mod redb;
// "locale:namespace.key":"Value"
pub trait KVStore {
    fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<()>;
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>>;
    fn delete(&mut self, locale: &str, key: &str) -> Result<()>;
    fn delete_locale(&mut self, locale: &str) -> Result<()>;
}

pub use redb::RedbStore;
