use async_trait::async_trait;
use redis::{
    AsyncCommands, Value,
    aio::ConnectionManager,
    streams::{StreamReadOptions, StreamReadReply},
};

use crate::{
    error::{QueueError, Result},
    message::{Delivery, Message},
    traits::consumer::Consumer,
};

use super::RedisStream;

fn field<T: redis::FromRedisValue>(
    fields: &std::collections::HashMap<String, Value>,
    key: &str,
) -> Result<T> {
    let value = fields.get(key).ok_or_else(|| {
        QueueError::InvalidMessage(format!("Missing required stream field: {key}"))
    })?;

    redis::from_redis_value_ref(value).map_err(|e| QueueError::InvalidMessage(e.to_string()))
}

fn decode_reply(reply: StreamReadReply) -> Result<Vec<Delivery>> {
    let mut deliveries = Vec::new();

    for stream in reply.keys {
        for entry in stream.ids {
            let payload: Vec<u8> = field(&entry.map, "payload")?;

            let headers_json: Option<String> = entry
                .map
                .get("headers")
                .map(redis::from_redis_value_ref)
                .transpose()
                .map_err(|e| QueueError::InvalidMessage(e.to_string()))?;

            let headers = match headers_json {
                Some(json) => serde_json::from_str(&json)
                    .map_err(|e| QueueError::InvalidMessage(e.to_string()))?,
                None => Default::default(),
            };

            let id: Option<String> = entry
                .map
                .get("message_id")
                .map(redis::from_redis_value_ref)
                .transpose()
                .map_err(|e| QueueError::InvalidMessage(e.to_string()))?;

            deliveries.push(Delivery {
                delivery_id: entry.id,
                message: Message {
                    id,
                    payload,
                    headers,
                },
            });
        }
    }

    Ok(deliveries)
}

#[async_trait]
impl Consumer for RedisStream {
    async fn consume(
        &self,
        destination: &str,
        group: &str,
        consumer: &str,
        count: usize,
        wait_ms: usize,
    ) -> Result<Vec<Delivery>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let mut conn: ConnectionManager = self.conn.clone();

        let mut options = StreamReadOptions::default()
            .group(group, consumer)
            .count(count);

        // In Redis Streams, BLOCK 0 means wait indefinitely.
        // Omitting BLOCK makes this call non-blocking.
        if wait_ms > 0 {
            options = options.block(wait_ms);
        }

        let reply: StreamReadReply = conn
            .xread_options(&[destination], &[">"], &options)
            .await
            .map_err(|e| QueueError::Consume(e.to_string()))?;

        decode_reply(reply)
    }

    async fn ack(&self, destination: &str, group: &str, delivery_id: &str) -> Result<()> {
        let mut conn = self.conn.clone();

        let _: i64 = conn
            .xack(destination, group, &[delivery_id])
            .await
            .map_err(|e| QueueError::Ack(e.to_string()))?;

        Ok(())
    }
}
