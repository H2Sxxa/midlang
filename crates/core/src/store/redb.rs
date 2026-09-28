use crate::store::{KVCursor, KVEntry, KVPage, SortOrder, StoreError, ValueState};

use super::KVStore;
use anyhow::Result;
use redb::{Database, ReadableDatabase, TableDefinition, TableError};
use std::{ops::Bound, path::Path, sync::Arc};
#[derive(Clone)]
pub struct RedbStore {
    db: Arc<Database>,
}

pub type RedbTableDefinition<'a> = TableDefinition<'a, String, String>;

impl KVStore for RedbStore {
    fn set(&self, locale: &str, key: &str, value: &str) -> Result<ValueState> {
        let tb = RedbTableDefinition::new(locale);
        let write_tx = self.db.begin_write()?;
        let previous = {
            let mut table = write_tx.open_table(tb)?;
            table
                .insert(key.to_string(), value.to_string())?
                .map(|val| val.value().to_string())
        };
        write_tx.commit()?;

        if let Some(old_value) = previous {
            return Ok(ValueState::Updated(old_value));
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

    fn delete(&self, locale: &str, key: &str) -> Result<String> {
        let tb = RedbTableDefinition::new(locale);
        let write_tx = self.db.begin_write()?;
        let removed = {
            let mut table = write_tx.open_table(tb)?;
            table
                .remove(key.to_string())?
                .map(|val| val.value().to_string())
        };

        match removed {
            Some(old_value) => {
                write_tx.commit()?;
                Ok(old_value)
            }
            // Dropping the write transaction aborts it, so a miss does not leave
            // an empty table behind.
            None => Err(StoreError::LocaleKeyNotExist(locale.to_string(), key.to_string()).into()),
        }
    }

    fn delete_locale(&self, locale: &str) -> Result<()> {
        let tb = RedbTableDefinition::new(locale);
        let write_tx = self.db.begin_write()?;
        write_tx.delete_table(tb)?;
        write_tx.commit()?;
        Ok(())
    }

    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage> {
        let tb = RedbTableDefinition::new(locale);
        let read_tx = self.db.begin_read()?;
        let table = match read_tx.open_table(tb) {
            Ok(table) => table,
            Err(TableError::TableDoesNotExist(_)) => {
                return Err(StoreError::LocaleNotExist(locale.to_string()).into());
            }
            Err(e) => return Err(e.into()),
        };

        // Resume the scan right after the key of the previous page.
        let bounds = match cursor.next.as_deref() {
            None => (Bound::Unbounded, Bound::Unbounded),
            Some(key) if cursor.order == SortOrder::Asc => {
                (Bound::Excluded(key.to_string()), Bound::Unbounded)
            }
            Some(key) => (Bound::Unbounded, Bound::Excluded(key.to_string())),
        };
        let mut range = table.range(bounds)?;

        let keyword = cursor.filter.as_deref().map(str::to_lowercase);
        let descending = cursor.order == SortOrder::Desc;
        let mut entries: Vec<KVEntry> = Vec::with_capacity(limit);
        let mut last: Option<String> = None;
        let mut more = false;

        // Read one match past the page to know whether a next page exists.
        while let Some(item) = if descending {
            range.next_back()
        } else {
            range.next()
        } {
            let (key_guard, value_guard) = item?;
            let key = key_guard.value().to_string();
            let value = value_guard.value().to_string();

            if let Some(keyword) = &keyword {
                if !key.to_lowercase().contains(keyword) && !value.to_lowercase().contains(keyword)
                {
                    continue;
                }
            }
            if entries.len() == limit {
                more = true;
                break;
            }
            last = Some(key.clone());
            entries.push(KVEntry { key, value });
        }

        Ok(KVPage {
            entries,
            next: if more {
                last.map(|key| KVCursor {
                    order: cursor.order,
                    filter: cursor.filter.clone(),
                    next: Some(key),
                })
            } else {
                None
            },
        })
    }
}

impl RedbStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        Ok(RedbStore {
            db: Arc::new(Database::create(path)?),
        })
    }
}
