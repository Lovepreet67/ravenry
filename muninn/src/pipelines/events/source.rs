use std::sync::Arc;

use async_trait::async_trait;
use iam_contracts::topic_registory::TopicRegistry; // ADJUST: real import path

use crate::core::StreamSource;
use crate::error::{Error, Result};

/// Discovers active tenant/topic streams from the registry. The ingester
/// calls this every reconcile interval, so new topics are picked up and
/// removed ones are dropped without a restart.
pub struct RegistrySource {
    registry: Arc<dyn TopicRegistry>,
}

impl RegistrySource {
    pub fn new(registry: Arc<dyn TopicRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl StreamSource for RegistrySource {
    async fn streams(&self) -> Result<Vec<String>> {
        let active = self
            .registry
            .list_active()
            .await
            .map_err(|e| Error::Source(e.to_string()))?;

        // Stream names come from the shared contract, never built by hand.
        Ok(active.iter().map(|t| t.destination()).collect())
    }
}
