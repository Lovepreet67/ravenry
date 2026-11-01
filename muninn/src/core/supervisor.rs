use std::future::Future;
use std::time::Duration;

use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Debug, PartialEq, Eq)]
pub enum Exit {
    /// SIGINT / SIGTERM. Normal shutdown.
    Signal,
    /// A worker stopped on its own. The process should exit non-zero
    /// so Docker (`restart: unless-stopped`) brings it back, instead of
    /// running as a "healthy" process that has stopped ingesting.
    TaskFailed,
}

pub struct Supervisor {
    shutdown: CancellationToken,
    tasks: JoinSet<String>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self { shutdown: CancellationToken::new(), tasks: JoinSet::new() }
    }

    /// Hand this to every worker.
    pub fn token(&self) -> CancellationToken {
        self.shutdown.clone()
    }

    pub fn spawn<F>(&mut self, name: impl Into<String>, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        self.tasks.spawn(async move {
            fut.await;
            name
        });
    }

    /// Blocks until a signal or any task exits, then cancels everyone and
    /// waits up to `grace` for in-flight batches to finish.
    pub async fn run(self, grace: Duration) -> Exit {
        let Self { shutdown, mut tasks } = self;

        let exit = tokio::select! {
            _ = wait_for_signal() => {
                info!("shutdown signal received");
                Exit::Signal
            }
            res = tasks.join_next() => {
                match res {
                    Some(Ok(name)) => error!(task = %name, "task exited unexpectedly"),
                    Some(Err(e)) => error!(error = %e, "task panicked"),
                    None => {}
                }
                Exit::TaskFailed
            }
        };

        shutdown.cancel();

        let drain = async {
            while let Some(res) = tasks.join_next().await {
                if let Err(e) = res {
                    error!(error = %e, "task panicked during shutdown");
                }
            }
        };
        if tokio::time::timeout(grace, drain).await.is_err() {
            // Leased rows and un-ACKed messages stay recoverable
            // (reaper and Redis redelivery), so aborting is safe.
            warn!("grace period exceeded, aborting remaining tasks");
            tasks.abort_all();
        }

        exit
    }
}

async fn wait_for_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}