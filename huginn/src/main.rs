mod api;
mod auth;
mod config;
mod db;
mod domain;
mod error;
mod repo;
mod state;

use std::sync::Arc;
use std::time::Duration;

use iam_contracts::auth::jwt::verifier::JwtVerifier;
use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::auth::keys::PublicKeyStore;
use crate::config::Config;
use crate::db::pool::PgReadiness;
use crate::repo::{PgAttemptRepository, PgDeliveryRepository, PgEventRepository};
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load()?;
    let shutdown_token = CancellationToken::new();
    // init mimir
    let mimir_handle = mimir::init(config.mimir, shutdown_token.child_token());
    tokio::spawn(async { mimir_handle.prom.await });

    // Fail fast if the database is unreachable.
    let pool = db::pool::connect(
        &config.database.url,
        config.database.max_connections,
        config.database.statement_timeout_ms,
    )
    .await?;

    let public_key = PublicKeyStore::fetch(&config.iam_config.public_key_url).await?;
    let jwt_verifier = Arc::new(JwtVerifier::from_pem(public_key.as_bytes())?);

    // The only place that names Postgres types. Swapping the backend
    // means changing these three lines.
    let state = AppState::new(
        jwt_verifier,
        Arc::new(PgEventRepository::new(pool.clone())),
        Arc::new(PgDeliveryRepository::new(pool.clone())),
        Arc::new(PgAttemptRepository::new(pool.clone())),
        Arc::new(PgReadiness(pool.clone())),
        config.access.payload_roles.clone(),
    );

    let app = api::router::build(state);

    let listener = tokio::net::TcpListener::bind(&&config.server.addr).await?;
    info!(addr = %&config.server.addr, "huginn listening");

    // Wait for in-flight requests on shutdown, but never hang forever.
    let grace = Duration::from_secs(config.server.shutdown_grace_secs);
    let server = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());
    tokio::select! {
        res = server => res?,
        _ = force_exit_after_signal(grace) => {
            tracing::warn!("grace period exceeded, exiting");
        }
    }

    pool.close().await;
    info!("huginn stopped");
    Ok(())
}

async fn shutdown_signal() {
    wait_for_signal().await;
    info!("shutdown signal received");
}

/// Fires `grace` after the signal, so a stuck connection can't block exit.
async fn force_exit_after_signal(grace: Duration) {
    wait_for_signal().await;
    tokio::time::sleep(grace).await;
}

async fn wait_for_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = ctrl_c.await;
    }
}
