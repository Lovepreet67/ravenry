mod admin;
mod consumer;
mod producer;

use redis::aio::ConnectionManager;

#[derive(Clone, Debug)]
pub struct RedisStream {
    pub(crate) conn: ConnectionManager,
}

impl RedisStream {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }
}
