use std::{path::Path, sync::Arc};

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use iam_contracts::auth::config::AuthConfig;
// src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub admin_api_key: String,
    pub server_port: u16,
    pub auth_config: AuthConfig,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Arc<Self>> {
        let config = Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("APP_").split("__"))
            .extract::<Self>()?;
        Ok(Arc::new(config))
    }
}
