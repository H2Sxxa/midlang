use crate::internals::{InternalService, issue::IssueEvent};

use super::store::KVStore;
use anyhow::Result;
use lru::LruCache;
use std::{num::NonZeroUsize, sync::Arc};

pub struct Translation<Store: KVStore> {
    store: Store,
    cache: LruCache<String, String>,
    internal_service: Arc<InternalService>,
}

impl<Store> Translation<Store>
where
    Store: KVStore,
{
    pub fn new(
        store: Store,
        cache_capacity: NonZeroUsize,
        internal_service: Arc<InternalService>,
    ) -> Self {
        Translation {
            store,
            cache: LruCache::new(cache_capacity),
            internal_service,
        }
    }

    pub async fn get(
        &mut self,
        locale: &str,
        namespace: &str,
        key: &str,
    ) -> Result<Option<String>> {
        self.get_key(locale, &format!("{}.{}", namespace, key))
            .await
    }

    pub async fn get_key(&mut self, locale: &str, key: &str) -> Result<Option<String>> {
        let cache_key = format!("{}:{}", locale, key);
        // Cache hit
        if let Some(value) = self.cache.get(&cache_key) {
            return Ok(Some(value.clone()));
        }
        // Cache miss, check store
        match self.store.get(locale, key)? {
            Some(value) => {
                self.cache.put(cache_key, value.clone());
                return Ok(Some(value));
            }
            None => {
                // Missing translation, report issue
                println!(
                    "Missing translation for locale '{}' and key '{}'",
                    locale, key
                );
                self.internal_service
                    .report_issue(IssueEvent::MissingTranslation {
                        locale: locale.to_string(),
                        key: key.to_string(),
                    })
                    .await?;
            }
        }

        Ok(None)
    }

    pub fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<()> {
        let cache_key = format!("{}:{}", locale, key);
        self.cache.put(cache_key, value.to_string());
        self.store.set(locale, key, value)?;
        Ok(())
    }
}
