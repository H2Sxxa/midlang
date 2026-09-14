use sqlx::{AnyPool, SqlitePool};

// Support PG/SQLite
pub mod changelog;
pub mod issues;
pub mod worker;

#[derive(Default)]
pub struct InternalService {
    issue_collector: Option<issues::IssueCollector>,
    sqlite_pool: Option<AnyPool>,
}

impl InternalService {
    pub fn issue_collector(&mut self, pool: AnyPool) {
        self.issue_collector = Some(issues::IssueCollector::new(pool));
    }

    pub fn sqlite_pool(&mut self, pool: AnyPool) {
        self.sqlite_pool = Some(pool);
    }

    pub fn issue_collector_sqlite() {
    }
}
