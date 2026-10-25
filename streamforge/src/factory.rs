use std::sync::Arc;

use crate::{config::QueueConfig, error::QueueError, traits::Queue};

pub async fn build_queue(config: &QueueConfig) -> Result<Arc<dyn Queue>, QueueError> {
    match config {
        QueueConfig::RedisStream(redis_config) => {
            let client = redis::Client::open(redis_config.url.as_str())
                .map_err(|e| QueueError::Connection(e.to_string()))?;

            let connection = client
                .get_connection_manager()
                .await
                .map_err(|e| QueueError::Connection(e.to_string()))?;

            Ok(Arc::new(crate::backends::redis_stream::RedisStream::new(
                connection,
            )))
        }
    }
}
