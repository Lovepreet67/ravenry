use std::time::Duration;

use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tracing::error;

use crate::platform::outbox::{handler, worker};

pub fn spawn(pool: PgPool, redis: ConnectionManager) -> JoinHandle<()> {
    tokio::spawn(async move {
        let handler_pool = pool.clone();

        worker::run(pool, 10, Duration::from_millis(500), move |event| {
            let pool = handler_pool.clone();
            let mut redis = redis.clone();

            async move { handler::process_tenant_projection(&pool, &mut redis, event).await }
        })
        .await;

        error!("Outbox worker stopped unexpectedly");
    })
}
