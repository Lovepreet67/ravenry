use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkerConfig {
    pub consumer_group: String,
    pub consumer_name: String,
    pub batch_size: usize,
    pub registry_refresh_interval_secs: u64,
    pub idle_interval_ms: u64,
}
