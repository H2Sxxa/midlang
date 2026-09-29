use std::{path::Path, sync::Arc};

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::internals::changelog::ChangelogRecorder;
use crate::store::StoreObserver;

pub mod changelog;
pub mod coverage;
pub mod issue;
pub mod worker;

/// Owns every derived-state service and presents one
/// [`StoreObserver`] to the store, so a store write reaches every service
/// through a single attachment point.
#[derive(Default)]
pub struct InternalService {
    pub issue: Option<Arc<issue::IssueCollector>>,
    pub changelog: Option<Arc<changelog::ChangelogRecorder>>,
    pub coverage: Option<Arc<coverage::CoverageReporter>>,
    workplace: Arc<worker::Workplace>,
}

impl InternalService {
    pub async fn conn(url: &str, issue: bool, changelog: bool, coverage: bool) -> Result<Self> {
        // Check if the database exists, if not, create it
        if !url.starts_with("sqlite://") && !Path::new(url).exists() {
            Sqlite::create_database(url).await?;
        }
        let pool = SqlitePool::connect(url).await?;

        let mut internal = Self::default();
        if issue {
            internal.issue = Some(Arc::new(issue::IssueCollector::new(pool.clone()).await?));
        }
        if changelog {
            internal.changelog = Some(Arc::new(
                changelog::ChangelogRecorder::new(pool.clone()).await?,
            ));
        }
        if coverage {
            internal.coverage = Some(Arc::new(
                coverage::CoverageReporter::new(pool.clone()).await?,
            ));
        }

        Ok(internal)
    }

    pub async fn issue_collector(mut self, pool: &SqlitePool) -> Result<Self> {
        self.issue = Some(Arc::new(issue::IssueCollector::new(pool.clone()).await?));
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

    pub async fn report_issue(&self, event: issue::IssueEvent) -> Result<()> {
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
