use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use anyhow::Result;

use crate::store::{
    KVCursor, KVEntry, KVPage, KVRead, KVStatistics, KVStore, ObserverRegistry, SortOrder,
    StoreError, StoreObserver, ValueState,
};

/// An in-memory KV store with the same observable semantics as [`super::RedbStore`].
///
/// The nested maps intentionally mirror the logical model: locales own their
/// keys, and a locale can exist even when its map is empty.
#[derive(Clone)]
pub struct MemStore {
    data: Arc<RwLock<BTreeMap<String, BTreeMap<String, String>>>>,
    observers: ObserverRegistry,
}

pub fn format_key(namespace: &str, key: &str) -> String {
    format!("{}.{}", namespace, key)
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
            observers: ObserverRegistry::default(),
        }
    }

    pub fn attach_observer(&mut self, observer: Arc<dyn StoreObserver>) {
        self.observers.replace(observer);
    }

    pub fn add_observer(&self, observer: Arc<dyn StoreObserver>) {
        self.observers.add(observer);
    }

    fn read_data(
        &self,
    ) -> std::sync::RwLockReadGuard<'_, BTreeMap<String, BTreeMap<String, String>>> {
        self.data.read().expect("memory store data poisoned")
    }

    fn write_data(
        &self,
    ) -> std::sync::RwLockWriteGuard<'_, BTreeMap<String, BTreeMap<String, String>>> {
        self.data.write().expect("memory store data poisoned")
    }
}

impl KVRead for MemStore {
    fn get(&self, locale: &str, key: &str) -> Result<Option<String>> {
        let data = self.read_data();
        let entries = data
            .get(locale)
            .ok_or_else(|| StoreError::LocaleNotExist(locale.to_owned()))?;
        Ok(entries.get(key).cloned())
    }

    fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage> {
        let data = self.read_data();
        let entries = data
            .get(locale)
            .ok_or_else(|| StoreError::LocaleNotExist(locale.to_owned()))?;

        if limit == 0 {
            return Ok(KVPage {
                items: Vec::new(),
                next: None,
            });
        }

        let keyword = cursor.filter.as_deref().map(str::to_lowercase);
        let mut matching: Vec<(&String, &String)> = entries
            .iter()
            .filter(|(key, _)| match (cursor.order, cursor.next.as_deref()) {
                (SortOrder::Asc, Some(after)) => key.as_str() > after,
                (SortOrder::Desc, Some(after)) => key.as_str() < after,
                (_, None) => true,
            })
            .filter(|(key, value)| {
                keyword.as_ref().is_none_or(|keyword| {
                    key.to_lowercase().contains(keyword) || value.to_lowercase().contains(keyword)
                })
            })
            .collect();

        if cursor.order == SortOrder::Desc {
            matching.reverse();
        }

        let more = matching.len() > limit;
        let items: Vec<KVEntry> = matching
            .into_iter()
            .take(limit)
            .map(|(key, value)| KVEntry {
                key: key.clone(),
                value: value.clone(),
            })
            .collect();
        let next = more.then(|| KVCursor {
            order: cursor.order,
            filter: cursor.filter.clone(),
            next: items.last().map(|entry| entry.key.clone()),
        });

        Ok(KVPage { items, next })
    }

    fn statistics(&self) -> Result<KVStatistics> {
        let data = self.read_data();
        let per_locale: BTreeMap<String, usize> = data
            .iter()
            .map(|(locale, entries)| (locale.clone(), entries.len()))
            .collect();
        let entries = per_locale.values().sum();

        Ok(KVStatistics {
            locales: per_locale.len(),
            entries,
            per_locale,
        })
    }
}

impl KVStore for MemStore {
    fn set(&self, locale: &str, key: &str, value: &str) -> Result<ValueState> {
        let previous = {
            let mut data = self.write_data();
            data.entry(locale.to_owned())
                .or_default()
                .insert(key.to_owned(), value.to_owned())
        };

        self.observers.notify_set(locale, key, value);
        Ok(match previous {
            Some(old_value) => ValueState::Updated(old_value),
            None => ValueState::Created,
        })
    }

    fn delete(&self, locale: &str, key: &str) -> Result<String> {
        let old_value = {
            let mut data = self.write_data();
            let entries = data
                .get_mut(locale)
                .ok_or_else(|| StoreError::LocaleNotExist(locale.to_owned()))?;
            entries
                .remove(key)
                .ok_or_else(|| StoreError::LocaleKeyNotExist(locale.to_owned(), key.to_owned()))?
        };

        self.observers.notify_delete(locale, key, &old_value);
        Ok(old_value)
    }

    fn delete_locale(&self, locale: &str) -> Result<()> {
        {
            let mut data = self.write_data();
            if data.remove(locale).is_none() {
                return Err(StoreError::LocaleNotExist(locale.to_owned()).into());
            }
        }

        self.observers.notify_delete_locale(locale);
        Ok(())
    }
}
