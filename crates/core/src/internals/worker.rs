use std::sync::Arc;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

#[async_trait::async_trait]
pub trait Workable {
    async fn work(&self) -> Result<WorkState>;
}

pub enum WorkState {
    ACTIVE,
    STOPPED,
}

pub struct Workplace {
    duration: std::time::Duration,
    cancel: CancellationToken,
}

impl Default for Workplace {
    fn default() -> Self {
        Self::new(std::time::Duration::from_secs(3))
    }
}

impl Workplace {
    pub fn new(duration: std::time::Duration) -> Self {
        Self {
            duration,
            cancel: CancellationToken::new(),
        }
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }

    pub fn go_work<W: Workable + Send + Sync + 'static>(
        self: &Arc<Workplace>,
        work: Arc<W>,
    ) -> tokio::task::JoinHandle<()> {
        let wp = self.clone();
        return tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = wp.cancel.cancelled() => break,
                    result = work.work() => {
                        if let Err(err) = result {
                            eprintln!("worker failed: {err:#}");
                            break;
                        }
                    }
                }
                tokio::select! {
                    _ = wp.cancel.cancelled() => break,
                    _ = tokio::time::sleep(wp.duration) => {}
                }
            }
        });
    }
}

#[cfg(test)]
mod workplace {
    use std::sync::{Arc, atomic::AtomicUsize};

    use crate::internals::worker::{WorkState, Workable, Workplace};

    struct Count10 {
        count: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl Workable for Count10 {
        async fn work(&self) -> anyhow::Result<WorkState> {
            println!(
                "Count: {}",
                self.count.load(std::sync::atomic::Ordering::Relaxed)
            );
            self.count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if self.count.load(std::sync::atomic::Ordering::Relaxed) == 10 {
                println!("Count reached 10, stopping work.");
                return Ok(WorkState::STOPPED);
            }
            Ok(WorkState::ACTIVE)
        }
    }
    #[tokio::test]
    async fn test_wp() {
        let wp = Arc::new(Workplace::default());
        let _ = wp
            .go_work(Arc::new(Count10 {
                count: AtomicUsize::new(0),
            }))
            .await;
    }
}
