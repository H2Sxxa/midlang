use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;

#[async_trait::async_trait]
pub trait Workable {
    async fn work(&mut self) -> Result<WorkState>;
}

pub enum WorkState {
    ACTIVE,
    STOPPED,
}

pub struct Workplace {
    duration: std::time::Duration,
    active: AtomicBool,
}

impl Workplace {
    pub fn new(duration: std::time::Duration) -> Self {
        Self {
            duration,
            active: AtomicBool::new(true),
        }
    }

    pub fn default() -> Self {
        Self::new(std::time::Duration::from_secs(3))
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn go_work<W: Workable + Send + Sync + 'static>(
        self: &Arc<Workplace>,
        mut work: W,
    ) -> tokio::task::JoinHandle<()> {
        let wp = self.clone();
        return tokio::spawn(async move {
            while let true = wp.active.load(Ordering::Relaxed) {
                tokio::time::sleep(wp.duration).await;
                // TODO: Handle errors from work.work() gracefully, maybe log them
                match work.work().await {
                    Ok(WorkState::ACTIVE) => continue,
                    Ok(WorkState::STOPPED) => break,
                    Err(e) => {
                        eprintln!("Work failed: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod workplace {
    use std::sync::Arc;

    use crate::internals::worker::{WorkState, Workable, Workplace};

    struct Count10 {
        count: usize,
    }

    #[async_trait::async_trait]
    impl Workable for Count10 {
        async fn work(&mut self) -> anyhow::Result<WorkState> {
            println!("Count: {}", self.count);
            self.count += 1;
            if self.count == 10 {
                println!("Count reached 10, stopping work.");
                return Ok(WorkState::STOPPED);
            }
            Ok(WorkState::ACTIVE)
        }
    }
    #[tokio::test]
    async fn test_wp() {
        let wp = Arc::new(Workplace::default());
        let _ = wp.go_work(Count10 { count: 0 }).await;
    }
}
