use std::sync::Arc;

use crate::platform::auth::keys::PublicKeyStore;
use iam_contracts::auth::jwt::verifier::JwtVerifier;

pub mod application;
pub mod config;
pub mod platform;
pub mod state;
pub mod transport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".into());
    let config = Arc::new(config::Config::load(config_path)?);
    let queue = streamforge::factory::build_queue(&config.queue).await?;
    let projections =
        platform::projection_store::ProjectionStore::connect(&config.projection.redis_url).await?;

    let publish_roles = config.iam.publish_roles.iter().cloned().collect();
    let public_key = PublicKeyStore::fetch(&config.iam.public_key_url).await?;
    let jwt_verifier = JwtVerifier::from_pem(public_key.as_bytes())?;
    let state = state::AppState {
        config: config.clone(),
        queue,
        publish_roles: Arc::new(publish_roles),
        jwt_verifier: Arc::new(jwt_verifier),
        projections,
    };

    let app = transport::http::router::get_router().with_state(state);

    let listener =
        tokio::net::TcpListener::bind((config.server.host.as_str(), config.server.port)).await?;

    tracing::info!(
        "inbound listening on {}:{}",
        config.server.host,
        config.server.port
    );

    axum::serve(listener, app).await?;

    Ok(())
}
