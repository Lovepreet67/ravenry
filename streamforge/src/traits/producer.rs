use std::fmt::Debug;

use async_trait::async_trait;

use crate::{error::Result, message::Message};

#[async_trait]
pub trait Producer: Send + Sync + Debug {
    async fn publish(&self, destination: &str, message: Message) -> Result<String>;
    async fn publish_batch(&self, messages: Vec<(&str, Message)>) -> Result<Vec<String>>;
}
