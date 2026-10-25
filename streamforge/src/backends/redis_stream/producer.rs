use async_trait::async_trait;

use crate::{
    error::{QueueError, Result},
    message::Message,
    traits::producer::Producer,
};

use super::RedisStream;

#[async_trait]
impl Producer for RedisStream {
    async fn publish(&self, destination: &str, message: Message) -> Result<String> {
        let mut conn = self.conn.clone();

        let headers = serde_json::to_string(&message.headers)
            .map_err(|e| QueueError::Publish(e.to_string()))?;

        let mut command = redis::cmd("XADD");
        command
            .arg(destination)
            .arg("*")
            .arg("payload")
            .arg(&message.payload)
            .arg("headers")
            .arg(headers);

        if let Some(id) = &message.id {
            command.arg("message_id").arg(id);
        }

        command
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| QueueError::Publish(e.to_string()))
    }
}
