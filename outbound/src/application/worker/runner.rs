use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio_util::sync::CancellationToken;
use tracing::warn;

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
pub enum WorkerCommand<A> {
    Add(A),
    Remove(A),
}

pub struct WorkerChannels<A, R> {
    pub cmd_rx: mpsc::Receiver<WorkerCommand<A>>,
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

    async fn apply_command(
        &mut self,
        command: WorkerCommand<W::Assignment>,
        assignments: &mut Vec<W::Assignment>,
        idx: &mut usize,
    ) {
        match command {
            WorkerCommand::Add(assignment) => {
                if !assignments.contains(&assignment) {
                    assignments.push(assignment);
                }
            }
            WorkerCommand::Remove(assignment) => {
                if let Some(pos) = assignments.iter().position(|e| e == &assignment) {
                    let removed = assignments.remove(pos);
                    self.worker.on_deprecate(removed).await;
                    if pos < *idx {
                        *idx -= 1;
                    }
                }
            }
        }
    }

    pub async fn run(mut self) {
        let mut assignments: Vec<W::Assignment> = Vec::new();
        let mut idx = 0usize;
        let mut idle_passes = 0u32;
        let mut any_work_this_pass = false;

        'outer: loop {
            if self.token.is_cancelled() {
                break;
            }

            // 1. Drain all pending commands without blocking.
            loop {
                match self.channels.cmd_rx.try_recv() {
                    Ok(command) => {
                        self.apply_command(command, &mut assignments, &mut idx)
                            .await
                    }
                    Err(TryRecvError::Empty) => break,
                    // Master is gone: nobody can assign or remove anything anymore.
                    Err(TryRecvError::Disconnected) => break 'outer,
                }
            }

            // 2. Nothing to do: block until a command arrives or we're cancelled.
            if assignments.is_empty() {
                idx = 0;
                idle_passes = 0;
                any_work_this_pass = false;

                let command = tokio::select! {
                    _ = self.token.cancelled() => break,
                    command = self.channels.cmd_rx.recv() => command,
                };
                match command {
                    Some(command) => {
                        self.apply_command(command, &mut assignments, &mut idx)
                            .await
                    }
                    None => break,
                }
                continue;
            }

            // 3. Round-robin: one assignment per tick.
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
                Err(err) => {
                    // Counts as "no work", so the idle sleep below doubles as backoff.
                    warn!(error = %format!("{err:#}"), "Worker step failed");
                }
            }

            // 4. After a full pass, decide whether to back off.
            if completed_full_pass {
                if !any_work_this_pass {
                    idle_passes = idle_passes.saturating_add(1);

                    // Reports are advisory: never let a full report channel block the worker.
                    if let Some(report) = self.worker.on_idle(idle_passes) {
                        let _ = self.channels.report_tx.try_send(report);
                    }

                    // Sleep, but wake immediately on a command so Add/Remove isn't
                    // delayed by up to `idle_sleep`.
                    let command = tokio::select! {
                        _ = self.token.cancelled() => break,
                        _ = tokio::time::sleep(self.idle_sleep) => None,
                        command = self.channels.cmd_rx.recv() => match command {
                            Some(command) => Some(command),
                            None => break,
                        },
                    };
                    if let Some(command) = command {
                        self.apply_command(command, &mut assignments, &mut idx)
                            .await;
                    }
                }
                any_work_this_pass = false;
            }
        }
    }
}
