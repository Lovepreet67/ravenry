pub mod iam;
pub mod projection;
pub mod server;

use std::{fs, path::Path};

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use iam::IamConfig;
use projection::ProjectionConfig;
use serde::Deserialize;
use server::ServerConfig;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub queue: streamforge::config::QueueConfig,
    pub iam: IamConfig,
    pub projection: ProjectionConfig,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        let config = Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("APP_").split("__"))
            .extract::<Self>()?;

        Ok(config)
    }
}
