use crate::store::{StoreError, ValueState};

use super::KVStore;
use anyhow::Result;
use redb::{Database, ReadableDatabase, TableDefinition, TableError};
use std::{path::Path, sync::Arc};
#[derive(Clone)]
pub struct RedbStore {
    db: Arc<Database>,
}

pub type RedbTableDefinition<'a> = TableDefinition<'a, String, String>;

impl KVStore for RedbStore {
    fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<ValueState> {
        let tb = RedbTableDefinition::new(locale);
        if let Some(val) = self
            .db
            .begin_write()?
            .open_table(tb)?
            .insert(key.to_string(), value.to_string())?
        {
            return Ok(ValueState::Updated(val.value().to_string()));
        }

        Ok(ValueState::Created)
    }

    fn get(&self, locale: &str, key: &str) -> Result<Option<String>> {
        let tb = RedbTableDefinition::new(locale);
        match self.db.begin_read()?.open_table(tb) {
            Ok(table) => {
                let value = table.get(key.to_string())?;
                Ok(value.map(|e| e.value().to_string()))
            }
            Err(TableError::TableDoesNotExist(_)) => {
                return Err(StoreError::LocaleNotExist(locale.to_string()).into());
            }
            Err(e) => {
                // Other errors, propagate the error
                Err(e.into())
            }
        }
    }

    fn delete(&mut self, locale: &str, key: &str) -> Result<String> {
        let tb = RedbTableDefinition::new(locale);
        match self
            .db
            .begin_write()?
            .open_table(tb)?
            .remove(key.to_string())?
        {
            Some(val) => Ok(val.value().to_string()),
            None => Err(StoreError::LocaleKeyNotExist(locale.to_string(), key.to_string()).into()),
        }
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
            db: Arc::new(Database::create(path)?),
        })
    }
}
