use std::time::Duration;

use tracing::info;

use super::{dlq_publisher::DlqPublisher, retry_worker::RetryWorker};

pub struct WorkerSupervisor {
    retry_worker: RetryWorker,
    dlq_publisher: DlqPublisher,
    retry_poll_interval: Duration,
    dlq_poll_interval: Duration,
}

impl WorkerSupervisor {
    pub fn new(
        retry_worker: RetryWorker,
        dlq_publisher: DlqPublisher,
        retry_poll_interval: Duration,
        dlq_poll_interval: Duration,
    ) -> Self {
        Self {
            retry_worker,
            dlq_publisher,
            retry_poll_interval: retry_poll_interval.max(Duration::from_millis(1)),
            dlq_poll_interval: dlq_poll_interval.max(Duration::from_millis(1)),
        }
    }

    /// Runs both background workers concurrently.
    ///
    /// The workers are intended to run for the lifetime of the service.
    pub async fn run(self) {
        info!(
            retry_poll_ms = self.retry_poll_interval.as_millis() as u64,
            dlq_poll_ms = self.dlq_poll_interval.as_millis() as u64,
            "Starting outbound background workers"
        );

        let retry = self.retry_worker.run(self.retry_poll_interval);
        let dlq = self.dlq_publisher.run(self.dlq_poll_interval);

        tokio::join!(retry, dlq);
    }
}
