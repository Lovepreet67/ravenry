use std::sync::Arc;

use serde::Deserialize;

fn default_retry_poll_interval_millis() -> u64 {
    500
}

fn default_dlq_poll_interval_millis() -> u64 {
    500
}

#[derive(Clone, Debug, Deserialize)]
pub struct StreamWorkerConfig {
    pub consumer_group: String,
    pub consumer_name_prefix: String,
    pub worker_per_stream: usize,
    pub batch_size: usize,
    pub idle_interval_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RetryWorkerConfig {
    pub batch_size: usize,
    pub worker_count: usize,

    #[serde(default = "default_retry_poll_interval_millis")]
    pub poll_interval_millis: u64,
}

#[derive(Debug, Deserialize, Clone)]

pub struct DlqWorkerConfig {
    pub batch_size: usize,
    pub worker_count: usize,

    #[serde(default = "default_dlq_poll_interval_millis")]
    pub poll_interval_millis: u64,
}

#[derive(Debug, Deserialize)]
pub struct WorkerConfig {
    pub registry_refresh_interval_secs: u64,
    pub stream_worker_config: StreamWorkerConfig,
    pub dlq_worker_config: DlqWorkerConfig,
    pub retry_worker_config: RetryWorkerConfig,
}

#[derive(Debug, Clone)]
pub struct WorkerRuntimeConfig {
    pub registry_refresh_interval_secs: u64,

    pub stream_worker_config: Arc<StreamWorkerConfig>,
    pub dlq_worker_config: Arc<DlqWorkerConfig>,
    pub retry_worker_config: Arc<RetryWorkerConfig>,
}

impl From<WorkerConfig> for WorkerRuntimeConfig {
    fn from(config: WorkerConfig) -> Self {
        Self {
            registry_refresh_interval_secs: config.registry_refresh_interval_secs,

            stream_worker_config: Arc::new(config.stream_worker_config),
            dlq_worker_config: Arc::new(config.dlq_worker_config),
            retry_worker_config: Arc::new(config.retry_worker_config),
        }
    }
}
