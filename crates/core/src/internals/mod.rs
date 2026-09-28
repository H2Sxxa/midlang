use std::{path::Path, sync::Arc};

use anyhow::Result;
use sqlx::{Sqlite, SqlitePool, migrate::MigrateDatabase};

use crate::internals::changelog::ChangelogRecorder;

pub mod changelog;
pub mod issue;
pub mod management;
pub mod worker;

#[derive(Default)]
pub struct InternalService {
    pub issue: Option<Arc<issue::IssueCollector>>,
    pub changelog: Option<Arc<changelog::ChangelogRecorder>>,
    workplace: Arc<worker::Workplace>,
}

impl InternalService {
    pub async fn conn(url: &str, issue: bool, changelog: bool) -> Result<Self> {
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
