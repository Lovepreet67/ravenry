use std::fmt::Debug;

use async_trait::async_trait;

use crate::error::Result;

#[async_trait]
pub trait QueueAdmin: Send + Sync + Debug {
    async fn ensure_group(&self, destination: &str, group: &str) -> Result<()>;
}
