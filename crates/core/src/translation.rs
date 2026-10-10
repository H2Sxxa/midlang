use crate::{
    internals::{
        InternalService,
        changelog::{ChangeRecord, Changelog, ChangelogCursor, ChangelogPage},
        issues::{IssueCursor, IssuePage, IssueRecord, IssueState, TranslationIssue},
    },
    store::{KVCursor, KVPage, KVRead, KVStore, StoreError, ValueState},
};

use anyhow::Result;
use std::sync::Arc;

/// Outcome of reverting the store to the state an entry records as its
/// predecessor.
#[derive(Debug)]
pub enum RollbackOutcome {
    /// The store was restored; carries the entry that was rolled back.
    Restored(ChangeRecord),
    /// The store no longer holds the value the entry produced, so reverting it
    /// would discard a later change.
    Conflict {
        /// Value the entry expects the store to hold.
        expected: Option<String>,
        /// Value the store holds now.
        actual: Option<String>,
    },
    /// No entry with that id exists.
    Missing,
}

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
                    .report_issue(TranslationIssue::MissingTranslation {
                        locale: locale.to_string(),
                        key: key.to_string(),
                    })
                    .await?;
                Ok(None)
            }
            Err(err) => {
                if let Some(StoreError::LocaleNotExist(locale)) = err.downcast_ref::<StoreError>() {
                    self.internal_service
                        .report_issue(TranslationIssue::MissingLocale {
                            locale: locale.clone(),
                        })
                        .await?;
                }

                Err(err)
            }
        }
    }

    pub fn statistics(&self) -> Result<crate::store::KVStatistics> {
        self.store.statistics()
    }

    /// Lists the entries for a locale using the store's keyset pagination and
    /// filter semantics. Listing is a pure read and deliberately does not
    /// report missing keys as issues.
    pub fn list(&self, locale: &str, cursor: &KVCursor, limit: usize) -> Result<KVPage> {
        self.store.list(locale, cursor, limit)
    }

    /// Lists reported translation issues, most recent first by default.
    pub async fn list_issues(&self, cursor: &IssueCursor, limit: usize) -> Result<IssuePage> {
        self.internal_service.list_issues(cursor, limit).await
    }

    /// Loads one reported issue by its id.
    pub async fn issue(&self, id: &str) -> Result<Option<IssueRecord>> {
        self.internal_service.issue(id).await
    }

    /// Marks one issue's state. Returns `false` when no such issue exists.
    pub async fn set_issue_state(&self, id: &str, state: IssueState) -> Result<bool> {
        self.internal_service.set_issue_state(id, state).await
    }

    /// Lists recorded changes, most recent first by default.
    pub async fn list_changes(
        &self,
        cursor: &ChangelogCursor,
        limit: usize,
    ) -> Result<ChangelogPage> {
        self.internal_service.list_changes(cursor, limit).await
    }

    /// Loads one recorded change by its id.
    pub async fn change(&self, id: i64) -> Result<Option<ChangeRecord>> {
        self.internal_service.change(id).await
    }
}

impl<Store> Translation<Store>
where
    Store: KVStore,
{
    // Will block on changelog reporting, so should be called in a separate task
    pub async fn set(&self, locale: &str, key: &str, value: &str) -> Result<()> {
        let change = match self.store.set(locale, key, value)? {
            ValueState::Created => {
                Changelog::created(locale.to_string(), key.to_string(), value.to_string())
            }
            // Rewriting the same value leaves the store unchanged, so there is
            // no change to record.
            ValueState::Updated(previous_value) if previous_value == value => return Ok(()),
            ValueState::Updated(previous_value) => Changelog::updated(
                locale.to_string(),
                key.to_string(),
                previous_value,
                value.to_string(),
            ),
            ValueState::Deleted(_) => {
                anyhow::bail!("store reported a deletion while setting '{locale}/{key}'")
            }
        };
        self.internal_service.report_change(change).await
    }

    pub async fn delete(&self, locale: &str, key: &str) -> Result<()> {
        let previous_value = self.store.delete(locale, key)?;
        self.internal_service
            .report_change(Changelog::deleted(
                locale.to_string(),
                key.to_string(),
                previous_value,
            ))
            .await
    }

    /// Reverts the store to the value an entry recorded as its predecessor.
    ///
    /// The rollback only applies while the store still holds what the entry
    /// produced; otherwise a later change would be silently discarded and the
    /// caller gets [`RollbackOutcome::Conflict`] instead. Reverting goes through
    /// [`Translation::set`] and [`Translation::delete`], so the rollback itself
    /// is recorded like any other change.
    pub async fn rollback(&self, id: i64) -> Result<RollbackOutcome> {
        let Some(entry) = self.internal_service.change(id).await? else {
            return Ok(RollbackOutcome::Missing);
        };
        let actual = self.store.get(&entry.locale, &entry.key)?;
        if actual != entry.new_value {
            return Ok(RollbackOutcome::Conflict {
                expected: entry.new_value.clone(),
                actual,
            });
        }

        match &entry.previous_value {
            Some(previous_value) => {
                self.set(&entry.locale, &entry.key, previous_value).await?;
            }
            None => {
                self.delete(&entry.locale, &entry.key).await?;
            }
        }
        Ok(RollbackOutcome::Restored(entry))
    }
}
