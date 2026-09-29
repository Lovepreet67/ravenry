use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::service::{database::get_db, redis::get_redis};

#[derive(Debug, Clone)]
pub struct AppState {
    pub redis_client: ConnectionManager,
    pub db_client: PgPool,
}

impl AppState {
    pub async fn new() -> Self {
        let rc = get_redis()
            .await
            .expect("Error while getting the redis Client");
        let dbc = get_db()
            .await
            .expect("Error while getting the database connection");
        Self {
            redis_client: rc,
            db_client: dbc,
        }
    }
}
