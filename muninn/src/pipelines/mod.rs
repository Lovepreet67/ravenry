use std::sync::Arc;

use iam_contracts::topic_registory::TopicRegistry; // ADJUST: real import path

use crate::core::Pipeline;

pub mod deliveries; // written next
pub mod events; // written next

pub fn all(registry: Arc<dyn TopicRegistry>) -> Vec<Arc<dyn Pipeline>> {
    vec![
        Arc::new(events::EventsPipeline::new(registry)),
        Arc::new(deliveries::DeliveriesPipeline::new()),
        // Future streams: one line each.
    ]
}
