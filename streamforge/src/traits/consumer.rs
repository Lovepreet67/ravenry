use std::fmt::Debug;

use async_trait::async_trait;

use crate::{error::Result, message::Delivery};

#[async_trait]
pub trait Consumer: Send + Sync + Debug {
    async fn consume(
        &self,
        destination: &str,
        group: &str,
        consumer: &str,
        count: usize,
        wait_ms: usize,
    ) -> Result<Vec<Delivery>>;

    async fn ack(&self, destination: &str, group: &str, delivery_id: &str) -> Result<()>;
}
