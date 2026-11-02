use serde::Deserialize;

fn default_retry_poll_interval_secs() -> u64 {
    1
}

fn default_dlq_poll_interval_secs() -> u64 {
    2
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerConfig {
    pub consumer_group: String,
    pub consumer_name: String,
    pub batch_size: usize,
    pub registry_refresh_interval_secs: u64,
    pub idle_interval_ms: u64,

    #[serde(default = "default_retry_poll_interval_secs")]
    pub retry_poll_interval_secs: u64,

    #[serde(default = "default_dlq_poll_interval_secs")]
    pub dlq_poll_interval_secs: u64,
}
