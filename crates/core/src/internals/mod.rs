use std::{path::Path, sync::Arc};

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::internals::changelog::ChangelogRecorder;
use crate::store::StoreObserver;

pub mod changelog;
pub mod coverage;
pub mod issues;
pub mod schema;
pub mod worker;

/// Selects which derived-state services are enabled for an [`InternalService`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ServiceOptions {
    pub issue: bool,
    pub changelog: bool,
    pub coverage: bool,
}

/// Owns every derived-state service and presents one
/// [`StoreObserver`] to the store, so a store write reaches every service
/// through a single attachment point.
#[derive(Default)]
pub struct InternalService {
    pub issue: Option<Arc<issues::IssueCollector>>,
    pub changelog: Option<Arc<changelog::ChangelogRecorder>>,
    pub coverage: Option<Arc<coverage::CoverageReporter>>,
    workplace: Arc<worker::Workplace>,
}

impl InternalService {
    pub async fn conn(url: &str, options: ServiceOptions) -> Result<Self> {
        // Check if the database exists, if not, create it
        if !url.starts_with("sqlite://") && !Path::new(url).exists() {
            Sqlite::create_database(url).await?;
        }
        let pool = SqlitePool::connect(url).await?;
        Self::conn_with_pool(pool, options).await
    }

    pub async fn conn_with_pool(pool: SqlitePool, options: ServiceOptions) -> Result<Self> {
        schema::ensure_tables(&pool).await?;

        let mut internal = Self::default();
        if options.issue {
            internal.issue = Some(Arc::new(issues::IssueCollector::new(pool.clone()).await?));
        }
        if options.changelog {
            internal.changelog = Some(Arc::new(
                changelog::ChangelogRecorder::new(pool.clone()).await?,
            ));
        }
        if options.coverage {
            internal.coverage = Some(Arc::new(
                coverage::CoverageReporter::new(pool.clone()).await?,
            ));
        }

        Ok(internal)
    }

    pub async fn issue_collector(mut self, pool: &SqlitePool) -> Result<Self> {
        self.issue = Some(Arc::new(issues::IssueCollector::new(pool.clone()).await?));
        Ok(self)
    }

    pub async fn changelog_recorder(mut self, pool: &SqlitePool) -> Result<Self> {
        self.changelog = Some(Arc::new(ChangelogRecorder::new(pool.clone()).await?));
        Ok(self)
    }

    pub fn service(self) -> Self {
        if let Some(issue_collector) = &self.issue {
            self.workplace.go_work(issue_collector.clone());
        }
        if let Some(coverage) = &self.coverage {
            self.workplace.go_work(coverage.clone());
        }
        self
    }

    pub async fn report_issue<E: issues::IssueEvent>(&self, event: E) -> Result<()> {
        if let Some(issue_collector) = &self.issue {
            issue_collector.report(event).await?;
        }
        Ok(())
    }

    pub async fn report_change(&self, change: changelog::Changelog) -> Result<()> {
        if let Some(changelog_recorder) = &self.changelog {
            changelog_recorder.report_change(change).await?;
        }
        Ok(())
    }

    /// Lists reported issues. An [InternalService] without an issue collector
    /// holds no issues, so the listing is empty rather than an error.
    pub async fn list_issues(
        &self,
        cursor: &issues::IssueCursor,
        limit: usize,
    ) -> Result<issues::IssuePage> {
        match &self.issue {
            Some(collector) => collector.list(cursor, limit).await,
            None => Ok(issues::IssuePage {
                items: Vec::new(),
                next: None,
            }),
        }
    }

    /// Loads one issue by its id.
    pub async fn issue(&self, id: &str) -> Result<Option<issues::IssueRecord>> {
        match &self.issue {
            Some(collector) => collector.get(id).await,
            None => Ok(None),
        }
    }

    /// Marks one issue's state. Returns `false` when no such issue exists.
    pub async fn set_issue_state(&self, id: &str, state: issues::IssueState) -> Result<bool> {
        match &self.issue {
            Some(collector) => collector.set_state(id, state).await,
            None => Ok(false),
        }
    }
}

impl StoreObserver for InternalService {
    fn on_set(&self, locale: &str, key: &str, value: &str) {
        if let Some(coverage) = &self.coverage {
            coverage.on_set(locale, key, value);
        }
    }

    fn on_delete(&self, locale: &str, key: &str, old_value: &str) {
        if let Some(coverage) = &self.coverage {
            coverage.on_delete(locale, key, old_value);
        }
    }

    fn on_delete_locale(&self, locale: &str) {
        if let Some(coverage) = &self.coverage {
            coverage.on_delete_locale(locale);
        }
    }
}
