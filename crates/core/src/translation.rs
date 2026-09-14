use std::num::NonZeroUsize;

use super::store::KVStore;
use lru::LruCache;

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

    pub fn get(&mut self, locale: &str, namespace: &str, key: &str) -> Option<String> {
        self.get_key(locale, &format!("{}.{}", namespace, key))
    }

    pub fn get_key(&mut self, locale: &str, key: &str) -> Option<String> {
        let cache_key = format!("{}:{}", locale, key);
        if let Some(value) = self.cache.get(&cache_key) {
            return Some(value.clone());
        }
        if let Ok(Some(value)) = self.store.get(locale, key) {
            self.cache.put(cache_key, value.clone());
            return Some(value);
        }
        // TODO Event: TranslationNotFound / Failed get translation from store
        None
    }
}
