use async_trait::async_trait;
use iam_contracts::{keys, streams};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    application::worker::runner::{WorkerChannels, WorkerRunner, WorkerTask},
    config::worker::RetryWorkerConfig,
    platform::redis::DeliveryStore,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryAssignement {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

pub enum RetryReport {
    Idle { worker_id: Uuid, idle_passes: u32 },
}

pub struct RetryWorker {
    id: Uuid,
    delivery_store: Arc<DeliveryStore>,
    config: Arc<RetryWorkerConfig>,
    token: CancellationToken,
}

pub struct RetryWorkerHandle {
    pub id: Uuid,
    pub add_tx: mpsc::Sender<RetryAssignement>,
    pub deprecate_tx: mpsc::Sender<RetryAssignement>,
    token: CancellationToken,
    handle: JoinHandle<()>,
}

impl RetryWorkerHandle {
    pub fn spawn(
        id: Uuid,
        delivery_store: Arc<DeliveryStore>,
        report_tx: mpsc::Sender<RetryReport>,
        channel_cap: usize,
        token: CancellationToken,
        config: Arc<RetryWorkerConfig>,
    ) -> Self {
        let (add_tx, add_rx) = mpsc::channel(channel_cap);
        let (deprecate_tx, deprecate_rx) = mpsc::channel(channel_cap);
        let idle_sleep = Duration::from_millis(config.idle_interval_millisecs);
        let worker = RetryWorker {
            id,
            delivery_store,
            config: config.clone(),
            token: token.clone(),
        };
        let channels = WorkerChannels {
            add_rx,
            deprecate_rx,
            report_tx,
        };
        let token_clone = token.clone();
        let handle = tokio::spawn(async move {
            info!(worker_id=%id,"Retry Worker started");
            WorkerRunner::new(worker, channels, idle_sleep, token_clone)
                .run()
                .await;
            info!(worker_id=%id,"Retry Worker stopped");
        });
        Self {
            id,
            add_tx,
            deprecate_tx,
            token,
            handle,
        }
    }
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
    pub async fn cancel(self) {
        self.token.cancel();
        if let Err(err) = self.handle.await {
            warn!(
                worker_id= %self.id,
                error = %err,
                "Retry worker join failed"
            );
        }
    }
}

#[async_trait]
impl WorkerTask for RetryWorker {
    type Assignment = RetryAssignement;
    type Report = RetryReport;
    async fn step(&mut self, assignement: &Self::Assignment) -> Result<bool, anyhow::Error> {
        let retry_key = keys::destination_retry(assignement.tenant_id, assignement.destination_id);
        let delivery_stream_key =
            streams::destination_deliveries(assignement.tenant_id, assignement.destination_id);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let moved = self
            .delivery_store
            .promote_due_retries(
                &retry_key,
                &delivery_stream_key,
                now_ms,
                self.config.batch_size,
            )
            .await?;
        Ok(moved > 0)
    }
    fn on_idle(&mut self, idle_passes: u32) -> Option<Self::Report> {
        None
        // Some(RetryReport::Idle {
        //     worker_id: self.id,
        //     idle_passes,
        // })
    }
}

pub struct RetryWorkerMaster {
    pub report_rx: mpsc::Receiver<RetryReport>,
    pub report_tx: mpsc::Sender<RetryReport>,
    pub workers: HashMap<Uuid, RetryWorkerHandle>,
    pub worker_to_destinations: HashMap<Uuid, HashSet<(Uuid, Uuid)>>,
    pub destination_to_worker: HashMap<(Uuid, Uuid), Uuid>,
}
