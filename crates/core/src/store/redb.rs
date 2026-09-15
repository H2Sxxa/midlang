use super::KVStore;
use anyhow::Result;
use redb::{Database, ReadableDatabase, TableDefinition, TableError};
use std::path::Path;

pub struct RedbStore {
    db: Database,
}

pub type RedbTableDefinition<'a> = TableDefinition<'a, String, String>;

impl KVStore for RedbStore {
    fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<()> {
        let tb = RedbTableDefinition::new(locale);
        self.db
            .begin_write()?
            .open_table(tb)?
            .insert(key.to_string(), value.to_string())?;
        Ok(())
    }

    fn get(&self, locale: &str, key: &str) -> Result<Option<String>> {
        let tb = RedbTableDefinition::new(locale);
        match self.db.begin_read()?.open_table(tb) {
            Ok(table) => {
                let value = table.get(key.to_string())?;
                Ok(value.map(|e| e.value().to_string()))
            }
            Err(TableError::TableDoesNotExist(_)) => {
                // Table does not exist, return None
                Ok(None)
            }
            Err(e) => {
                // Other errors, propagate the error
                Err(e.into())
            }
        }
    }

    fn delete(&mut self, locale: &str, key: &str) -> Result<()> {
        let tb = RedbTableDefinition::new(locale);
        self.db
            .begin_write()?
            .open_table(tb)?
            .remove(key.to_string())?;
        Ok(())
    }

    fn delete_locale(&mut self, locale: &str) -> Result<()> {
        let tb = RedbTableDefinition::new(locale);
        self.db.begin_write()?.delete_table(tb)?;
        Ok(())
    }
}

impl RedbStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(RedbStore {
            db: Database::create(path)?,
        })
    }
}
