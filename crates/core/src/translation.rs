use super::store::KVStore;
use anyhow::Result;
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct Translation<Store: KVStore> {
    store: Store,
    cache: LruCache<String, String>,
}

impl<Store> Translation<Store>
where
    Store: KVStore,
{
    pub fn new(store: Store, cache_capacity: NonZeroUsize) -> Self {
        Translation {
            store,
            cache: LruCache::new(cache_capacity),
        }
    }

    pub fn get(&mut self, locale: &str, namespace: &str, key: &str) -> Result<Option<String>> {
        self.get_key(locale, &format!("{}.{}", namespace, key))
    }

    pub fn get_key(&mut self, locale: &str, key: &str) -> Result<Option<String>> {
        let cache_key = format!("{}:{}", locale, key);
        if let Some(value) = self.cache.get(&cache_key) {
            return Ok(Some(value.clone()));
        }
        if let Some(value) = self.store.get(locale, key)? {
            self.cache.put(cache_key, value.clone());
            return Ok(Some(value));
        }
        // TODO Event: TranslationNotFound / Failed get translation from store
        Ok(None)
    }

    pub fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<()> {
        let cache_key = format!("{}:{}", locale, key);
        self.cache.put(cache_key, value.to_string());
        self.store.set(locale, key, value)?;
        Ok(())
    }
}
