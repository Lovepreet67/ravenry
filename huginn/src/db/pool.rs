use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

/// Read-only pool. Two layers of protection against writes:
/// 1. The DB role should only have SELECT (set up on the Muninn side).
/// 2. Every connection defaults to read-only transactions, in case the
///    role is ever granted too much.
///
/// `statement_timeout_ms` stops a heavy dashboard query from running
/// forever and holding a connection.
pub async fn connect(
    url: &str,
    max_connections: u32,
    statement_timeout_ms: u64,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET default_transaction_read_only = on")
                    .execute(&mut *conn)
                    .await?;
                // SET does not accept bind parameters; the value is a u64.
                sqlx::query(&format!("SET statement_timeout = {statement_timeout_ms}"))
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
}

use async_trait::async_trait;

use crate::repo::Readiness;

pub struct PgReadiness(pub PgPool);

#[async_trait]
impl Readiness for PgReadiness {
    async fn check(&self) -> Result<(), String> {
        sqlx::query("SELECT 1")
            .execute(&self.0)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}
