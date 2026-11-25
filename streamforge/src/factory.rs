use std::{sync::Arc, time::Duration};

use redis::aio::{ConnectionManager, ConnectionManagerConfig};

use crate::{config::QueueConfig, error::QueueError, traits::Queue};

pub async fn build_queue(config: &QueueConfig) -> Result<Arc<dyn Queue>, QueueError> {
    match config {
        QueueConfig::RedisStream(redis_config) => {
            let client = redis::Client::open(redis_config.url.as_str())
                .map_err(|e| QueueError::Connection(e.to_string()))?;

            let cfg = ConnectionManagerConfig::new()
                .set_connection_timeout(Some(Duration::from_secs(5)))
                .set_response_timeout(Some(Duration::from_secs(10)));
            let connection = ConnectionManager::new_with_config(client, cfg)
                .await
                .map_err(|e| QueueError::Connection(e.to_string()))?;

            Ok(Arc::new(crate::backends::redis_stream::RedisStream::new(
                connection,
            )))
        }
    }
}
