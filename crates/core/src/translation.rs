use crate::{
    internals::{
        InternalService,
        changelog::{ChangeState, Changelog},
        issue::IssueEvent,
    },
    store::{KVRead, KVStore, StoreError, ValueState},
};

use anyhow::Result;
use std::sync::Arc;

/// Reads go through [`KVRead`] and writes through [`KVStore`], so a read-only
/// store (a version snapshot) can be served by the same read path and simply
/// has no write methods to offer.
#[derive(Clone)]
pub struct Translation<Store: KVRead> {
    store: Store,
    internal_service: Arc<InternalService>,
}

impl<Store> Translation<Store>
where
    Store: KVRead,
{
    pub fn new(store: Store, internal_service: Arc<InternalService>) -> Self {
        Translation {
            store,
            internal_service,
        }
    }

    pub async fn get(&self, locale: &str, namespace: &str, key: &str) -> Result<Option<String>> {
        self.get_key(locale, &format!("{}.{}", namespace, key))
            .await
    }

    pub async fn get_key(&self, locale: &str, key: &str) -> Result<Option<String>> {
        match self.store.get(locale, key) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => {
                // Missing translation, report issue
                self.internal_service
                    .report_issue(IssueEvent::MissingTranslation {
                        locale: locale.to_string(),
                        key: key.to_string(),
                    })
                    .await?;
                Ok(None)
            }
            Err(err) => {
                if let Some(StoreError::LocaleNotExist(locale)) = err.downcast_ref::<StoreError>() {
                    self.internal_service
                        .report_issue(IssueEvent::MissingLocale {
                            locale: locale.clone(),
                        })
                        .await?;
                }

                Err(err)
            }
        }
    }
}

impl<Store> Translation<Store>
where
    Store: KVStore,
{
    // Will block on changelog reporting, so should be called in a separate task
    pub async fn set(&self, locale: &str, key: &str, value: &str) -> Result<()> {
        if let ValueState::Updated(old_value) = self.store.set(locale, key, value)? {
            self.internal_service
                .report_change(Changelog::new(
                    locale.to_string(),
                    key.to_string(),
                    old_value.clone(),
                    ChangeState::Updated,
                ))
                .await?;
        }
        Ok(())
    }

    pub async fn delete(&self, locale: &str, key: &str) -> Result<()> {
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
        Ok(())
    }
}
