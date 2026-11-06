use std::sync::Arc;

use iam_contracts::topic_registory::TopicRegistry; // ADJUST: real import path

use crate::core::Pipeline;

pub mod deliveries;
pub mod events;

pub fn all(registry: Arc<dyn TopicRegistry>) -> Vec<Arc<dyn Pipeline>> {
    return vec![
        Arc::new(events::EventsPipeline::new(registry)),
        Arc::new(deliveries::DeliveriesPipeline::new()),
        // Future streams: one line each.
    ];
}
