use figment::providers::{Env, Format, Toml};
use figment::Figment;
use serde::Deserialize;
use streamforge::config::QueueConfig;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub queue: QueueConfig,
    #[serde(default)]
    pub inbox: InboxConfig,
    #[serde(default)]
    pub workers: WorkersConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "d_max_conn")]
    pub max_connections: u32,
    #[serde(default = "d_min_conn")]
    pub min_connections: u32,
}
fn d_max_conn() -> u32 {
    10
}
fn d_min_conn() -> u32 {
    2
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InboxConfig {
    pub batch_size: i64,
    pub lease_secs: u64,
    pub poll_interval_ms: u64,
    pub max_attempts: i32,
    pub initial_backoff_secs: u64,
    pub max_backoff_secs: u64,
    pub reaper_interval_secs: u64,
}
impl Default for InboxConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            lease_secs: 60,
            poll_interval_ms: 500,
            max_attempts: 10,
            initial_backoff_secs: 5,
            max_backoff_secs: 300,
            reaper_interval_secs: 15,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkersConfig {
    /// Processor tasks PER pipeline.
    pub processors: usize,
    pub shutdown_grace_secs: u64,
}
impl Default for WorkersConfig {
    fn default() -> Self {
        Self {
            processors: 2,
            shutdown_grace_secs: 30,
        }
    }
}

impl Config {
    /// config.toml (or $CONFIG_PATH), overridden by env vars like
    /// MUNINN_DATABASE__URL and MUNINN_REDIS__URL (double underscore
    /// separates nesting). Secrets go in env, not in the file.
    pub fn load() -> Result<Self, figment::Error> {
        let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".into());
        Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("MUNINN_").split("__"))
            .extract()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ObservabilityConfig {
    pub http_addr: String,
    pub gauge_interval_secs: u64,
}
impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            http_addr: "0.0.0.0:8080".into(),
            gauge_interval_secs: 15,
        }
    }
}
