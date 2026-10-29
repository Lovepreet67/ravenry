use axum::Router;
use iam_contracts::auth::jwt::{signer::JwtSigner, verifier::JwtVerifier};
use sqlx::postgres::PgPoolOptions;
use std::{format, sync::Arc};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::{
    config::Config,
    domain::{
        destination::repository::DatabaseDestinationRepository,
        membership::repository::DatabaseMembershipRepository,
        role::repository::DatabaseRoleRepository, tenant::repository::DatabaseTenantRepository,
        topic::repository::DatabaseTopicRepository, user::repository::DatabaseUserRepository,
    },
    http::router,
    platform::{
        outbox::{handler, worker},
        projection::resync,
        redis_provider,
    },
    state::AppState,
};
use std::time::Duration;

pub mod auth;
pub mod config;
pub mod domain;
pub mod http;
pub mod platform;
pub mod shared;
pub mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;
    let db_conn = PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|_conn, _meta| {
            Box::pin(async move {
                tracing::info!("db connection aquired");
                Ok(())
            })
        })
        .connect(&config.database_url)
        .await?;
    let mut redis = redis_provider::connect(&config.redis_url).await?;
    match resync::resync(&db_conn, &mut redis).await {
        Ok(_) => info!("resync successfull"),
        Err(e) => error!("Working on the error {e}"),
    };

    // Start background processing.
    let _outbox_worker = platform::worker::outbox_worker::spawn(db_conn.clone(), redis.clone());
    let tenant_repo = DatabaseTenantRepository::new(db_conn.clone());
    let user_repo = DatabaseUserRepository::new(db_conn.clone());
    let membership_repo = DatabaseMembershipRepository::new(db_conn.clone());
    let role_repo = DatabaseRoleRepository::new(db_conn.clone());
    let destination_repo = DatabaseDestinationRepository::new(db_conn.clone());
    let topic_repo = DatabaseTopicRepository::new(db_conn);
    let jwt_signer = JwtSigner::from_pem(include_bytes!("../keys/private.pem"))?;
    let jwt_verifier = JwtVerifier::from_pem(include_bytes!("../keys/public.pem"))?;
    let state = AppState {
        tenant_repo: Arc::new(tenant_repo),
        user_repo: Arc::new(user_repo),
        membership_repo: Arc::new(membership_repo),
        role_repo: Arc::new(role_repo),
        destination_repo: Arc::new(destination_repo),
        topic_repo: Arc::new(topic_repo),
        config: config.clone(),
        jwt_verifier: Arc::new(jwt_verifier),
        jwt_signer: Arc::new(jwt_signer),
    };
    let app: Router<()> = router().with_state(state);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
