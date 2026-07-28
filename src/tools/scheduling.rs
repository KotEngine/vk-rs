//! Simple delayed task scheduler

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use tokio::task::JoinHandle;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Schedule a one-shot delayed task
pub fn run_later<F>(delay: Duration, f: F) -> JoinHandle<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        f.await;
    })
}

/// Schedule a repeating task until cancelled
pub fn run_interval<F, Fut>(period: Duration, mut f: F) -> JoinHandle<()>
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(period);
        loop {
            interval.tick().await;
            f().await;
        }
    })
}

/// Delayed task builder
pub struct DelayedTask {
    delay: Duration,
    task: Option<Box<dyn FnOnce() -> BoxFuture + Send>>,
}

impl DelayedTask {
    pub fn after(delay: Duration) -> Self {
        Self {
            delay,
            task: None,
        }
    }

    pub fn run<F, Fut>(mut self, f: F) -> JoinHandle<()>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.task = Some(Box::new(move || Box::pin(f())));
        let delay = self.delay;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            if let Some(task) = self.task {
                task().await;
            }
        })
    }
}
