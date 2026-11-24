use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait WorkerTask: Send + 'static {
    type Assignment: Eq + Clone + Send + Sync + 'static;
    type Report: Send + 'static;

    /// Executes one batch step for `assignment`.
    /// Returns `Ok(true)` if work was processed, `Ok(false)` if no work was available.
    async fn step(&mut self, assignment: &Self::Assignment) -> Result<bool, anyhow::Error>;

    /// Called when an assignment is deprecated; finish any in-flight work before removing.
    async fn on_deprecate(&mut self, _assignment: Self::Assignment) {}

    /// Builds an optional report after a full pass across all current assignments had no work.
    fn on_idle(&mut self, _idle_passes: u32) -> Option<Self::Report> {
        None
    }
}

pub struct WorkerChannels<A, R> {
    pub add_rx: mpsc::Receiver<A>,
    pub deprecate_rx: mpsc::Receiver<A>,
    pub report_tx: mpsc::Sender<R>,
}

pub struct WorkerRunner<W: WorkerTask> {
    worker: W,
    channels: WorkerChannels<W::Assignment, W::Report>,
    idle_sleep: Duration,
    token: CancellationToken,
}

impl<W: WorkerTask> WorkerRunner<W> {
    pub fn new(
        worker: W,
        channels: WorkerChannels<W::Assignment, W::Report>,
        idle_sleep: Duration,
        token: CancellationToken,
    ) -> Self {
        Self {
            worker,
            channels,
            idle_sleep,
            token,
        }
    }

    pub async fn run(mut self) {
        let mut assignments: Vec<W::Assignment> = Vec::new();
        let mut idx = 0usize;
        let mut idle_passes = 0u32;
        let mut any_work_this_pass = false;

        loop {
            if self.token.is_cancelled() {
                break;
            }

            // 1. Apply pending additions and deprecations non-blockingly
            while let Ok(item) = self.channels.add_rx.try_recv() {
                if !assignments.contains(&item) {
                    assignments.push(item);
                }
            }
            while let Ok(item) = self.channels.deprecate_rx.try_recv() {
                if let Some(pos) = assignments.iter().position(|existing| existing == &item) {
                    let removed = assignments.remove(pos);
                    self.worker.on_deprecate(removed).await;
                    if pos < idx {
                        idx = idx.saturating_sub(1);
                    }
                }
            }

            // 2. Wait for work or cancellation when no assignments are active
            if assignments.is_empty() {
                tokio::select! {
                    _ = self.token.cancelled() => break,
                    maybe_item = self.channels.add_rx.recv() => {
                        match maybe_item {
                            Some(item) => {
                                assignments.push(item);
                                idx = 0;
                                idle_passes = 0;
                                any_work_this_pass = false;
                            }
                            None => break,
                        }
                    }
                }
                continue;
            }

            // 3. Round-robin one assignment per tick
            if idx >= assignments.len() {
                idx = 0;
            }
            let current = assignments[idx].clone();
            idx = (idx + 1) % assignments.len();
            let completed_full_pass = idx == 0;

            match self.worker.step(&current).await {
                Ok(true) => {
                    any_work_this_pass = true;
                    idle_passes = 0;
                }
                Ok(false) => {}
                Err(_) => {}
            }

            // 4. After a full pass over all current assignments, evaluate idleness
            if completed_full_pass {
                if !any_work_this_pass {
                    idle_passes += 1;
                    if let Some(report) = self.worker.on_idle(idle_passes) {
                        tokio::select! {
                            _ = self.token.cancelled() => break,
                            _ = self.channels.report_tx.send(report) => {}
                        }
                    }
                    tokio::select! {
                        _ = self.token.cancelled() => break,
                        _ = tokio::time::sleep(self.idle_sleep) => {}
                    }
                }
                any_work_this_pass = false;
            }
        }
    }
}
