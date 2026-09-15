use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;

use crate::internals::changelog::ChangelogRecorder;

pub mod changelog;
pub mod issue;
pub mod worker;

#[derive(Default)]
pub struct InternalService {
    pub issue: Option<Arc<issue::IssueCollector>>,
    pub changelog: Option<Arc<changelog::ChangelogRecorder>>,
    workplace: Arc<worker::Workplace>,
}

impl InternalService {
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
