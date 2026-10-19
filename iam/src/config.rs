use std::sync::Arc;

// src/config.rs
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub admin_api_key: String,
    pub jwt_ttl_hours: u64,
    pub server_port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Config {
            database_url: std::env::var("DATABASE_URL")?,
            redis_url: std::env::var("REDIS_URL")?,
            admin_api_key: std::env::var("ADMIN_API_KEY")?,
            jwt_ttl_hours: std::env::var("JWT_TTL_HOURS")
                .unwrap_or_else(|_| "24".into())
                .parse()?,
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".into())
                .parse()?,
        }))
    }
}
