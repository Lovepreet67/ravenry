use axum::Router;
use sqlx::postgres::PgPoolOptions;
use std::{format, sync::Arc};
use tokio::net::TcpListener;

use crate::{
    config::Config,
    domain::{
        membership::repository::DatabaseMembershipRepository,
        role::repository::DatabaseRoleRepository, tenant::repository::DatabaseTenantRepository,
        user::repository::DatabaseUserRepository,
    },
    http::router,
    state::AppState,
};

pub mod auth;
pub mod config;
pub mod domain;
pub mod http;
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
    let tenant_repo = DatabaseTenantRepository::new(db_conn.clone());
    let user_repo = DatabaseUserRepository::new(db_conn.clone());
    let membership_repo = DatabaseMembershipRepository::new(db_conn.clone());
    let role_repo = DatabaseRoleRepository::new(db_conn);
    let state = AppState {
        tenant_repo: Arc::new(tenant_repo),
        user_repo: Arc::new(user_repo),
        membership_repo: Arc::new(membership_repo),
        role_repo: Arc::new(role_repo),
        config: config.clone(),
    };
    let app: Router<()> = router().with_state(state);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.server_port)).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
