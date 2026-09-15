use std::sync::Arc;

use anyhow::Result;
use sqlx::SqlitePool;

pub mod changelog;
pub mod issue;
pub mod worker;

#[derive(Default)]
pub struct InternalService {
    pub issue: Option<Arc<issue::IssueCollector>>,
    pool: Option<SqlitePool>,
    workplace: Arc<worker::Workplace>,
}

impl InternalService {
    pub fn issue_collector(mut self, pool: SqlitePool) -> Self {
        self.pool = Some(pool.clone());
        self.issue = Some(Arc::new(issue::IssueCollector::new(pool)));
        self
    }

    pub async fn issue_collector_sqlitepath(mut self, path: &str) -> Result<Self> {
        let pool = SqlitePool::connect(path).await?;
        self.issue = Some(Arc::new(issue::IssueCollector::new(pool)));
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
}
