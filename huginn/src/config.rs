use figment::Figment;
use figment::providers::{Env, Format, Toml};
use mimir::config::MimirConfig;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub access: AccessConfig,
    pub iam_config: IamConfig,
    pub mimir: MimirConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// Must use the read-only role (huginn_ro). Keep the password in env.
    pub url: String,
    #[serde(default = "d_max_conn")]
    pub max_connections: u32,
    /// Stops a heavy dashboard query from holding a connection forever.
    #[serde(default = "d_stmt_timeout")]
    pub statement_timeout_ms: u64,
}
fn d_max_conn() -> u32 {
    10
}
fn d_stmt_timeout() -> u64 {
    5_000
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub addr: String,
    pub shutdown_grace_secs: u64,
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:8081".into(),
            shutdown_grace_secs: 15,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AccessConfig {
    /// Role slugs allowed to see event payloads. Everyone else with a
    /// membership sees metadata only. Admin always sees payloads.
    pub payload_roles: Vec<String>,
}
impl Default for AccessConfig {
    fn default() -> Self {
        Self {
            payload_roles: vec!["owner".into(), "admin".into()],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IamConfig {
    pub public_key_url: String,
    #[serde(default = "d_http_timeout")]
    pub http_timeout_secs: u64,
}
fn d_http_timeout() -> u64 {
    5
}
fn d_cache_ttl() -> u64 {
    60
}

impl Config {
    /// config.toml (or $CONFIG_PATH), overridden by env vars such as
    /// HUGINN_DATABASE__URL. A double underscore separates nesting.
    pub fn load() -> Result<Self, figment::Error> {
        let path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".into());
        Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("HUGINN_").split("__"))
            .extract()
    }
}
