use anyhow::Result;
use sqlx::SqlitePool;

pub mod changelog;
pub mod issue;
pub mod worker;

#[derive(Default)]
pub struct InternalService {
    issue: Option<issue::IssueCollector>,
    pool: Option<SqlitePool>,
}

impl InternalService {
    pub fn issue_collector(&mut self, pool: SqlitePool) -> &mut Self {
        self.pool = Some(pool.clone());
        self.issue = Some(issue::IssueCollector::new(pool));
        self
    }

    pub async fn issue_collector_sqlitepath(&mut self, path: &str) -> Result<&mut Self> {
        let pool = SqlitePool::connect(path).await?;
        self.issue = Some(issue::IssueCollector::new(pool));
        Ok(self)
    }
}
