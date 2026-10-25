use serde::Deserialize;

pub mod redis_stream;

use redis_stream::RedisStreamConfig;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case")]
pub enum QueueConfig {
    RedisStream(RedisStreamConfig),
    // RabbitMq(RabbitMqConfig),
}
