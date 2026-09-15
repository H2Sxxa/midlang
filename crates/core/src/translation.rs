use crate::{
    internals::{
        InternalService,
        changelog::{ChangeState, Changelog},
        issue::IssueEvent,
    },
    store::{StoreError, ValueState},
};

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
        match self.store.get(locale, key) {
            Ok(value) => match value {
                Some(value) => {
                    self.cache.put(cache_key, value.clone());
                    return Ok(Some(value));
                }
                None => {
                    // Missing translation, report issue
                    self.internal_service
                        .report_issue(IssueEvent::MissingTranslation {
                            locale: locale.to_string(),
                            key: key.to_string(),
                        })
                        .await?;
                }
            },
            Err(err) => {
                if let Some(store_err) = err.downcast_ref::<StoreError>() {
                    match store_err {
                        // Missing locale, report issue
                        StoreError::LocaleNotExist(locale) => {
                            self.internal_service
                                .report_issue(IssueEvent::MissingLocale {
                                    locale: locale.clone(),
                                })
                                .await?;
                        }
                        _ => (),
                    }
                }

                return Err(err);
            }
        }

        Ok(None)
    }

    // Will block on changlog reporting, so should be called in a separate task
    pub async fn set(&mut self, locale: &str, key: &str, value: &str) -> Result<()> {
        let cache_key = format!("{}:{}", locale, key);
        self.cache.put(cache_key, value.to_string());
        match self.store.set(locale, key, value)? {
            ValueState::Updated(old_value) => {
                self.internal_service
                    .report_change(Changelog::new(
                        locale.to_string(),
                        key.to_string(),
                        old_value.clone(),
                        ChangeState::Updated,
                    ))
                    .await?;
            }
            _ => (),
        }
        Ok(())
    }

    pub async fn delete(&mut self, locale: &str, key: &str) -> Result<()> {
        let old_value = self.store.delete(locale, key)?;
        // Report deletion to changelog
        self.internal_service
            .report_change(Changelog::new(
                locale.to_string(),
                key.to_string(),
                old_value.clone(),
                ChangeState::Deleted,
            ))
            .await?;
        // Clear cache after deletion to ensure that the next get will not return a stale value
        let cache_key = format!("{}:{}", locale, key);
        self.cache.pop(&cache_key);
        Ok(())
    }
}
