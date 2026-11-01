use std::sync::Arc;

use sqlx::PgPool;
use streamforge::traits::Queue; // ADJUST: real import path

use crate::core::inbox::InboxStore;

/// Built once at startup, shared via Arc. One pool, one Redis connection.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub queue: Arc<dyn Queue>,
    pub inbox: Arc<dyn InboxStore>,
}
