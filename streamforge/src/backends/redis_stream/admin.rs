use async_trait::async_trait;
use redis::aio::ConnectionManager;

use crate::{
    error::{QueueError, Result},
    traits::admin::QueueAdmin,
};

use super::RedisStream;

#[async_trait]
impl QueueAdmin for RedisStream {
    async fn ensure_group(&self, destination: &str, group: &str) -> Result<()> {
        let mut conn: ConnectionManager = self.conn.clone();

        let result: std::result::Result<String, redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(destination)
            .arg(group)
            .arg("$")
            .arg("MKSTREAM")
            .query_async(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(error) if error.code() == Some("BUSYGROUP") => Ok(()),
            Err(error) => Err(QueueError::Setup(error.to_string())),
        }
    }
}
