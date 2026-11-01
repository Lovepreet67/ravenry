mod model;
mod projector;
mod source;

use std::sync::Arc;

use iam_contracts::topic_registory::TopicRegistry;

use crate::core::inbox::NewInboxMessage;
use crate::core::{Pipeline, Projector, StreamSource};

pub struct EventsPipeline {
    source: Arc<dyn StreamSource>,
    projector: Arc<dyn Projector>,
}

impl EventsPipeline {
    pub fn new(registry: Arc<dyn TopicRegistry>) -> Self {
        Self {
            source: Arc::new(source::RegistrySource::new(registry)),
            projector: Arc::new(projector::EventsProjector),
        }
    }
}

impl Pipeline for EventsPipeline {
    fn name(&self) -> &'static str {
        model::SOURCE
    }

    fn consumer_group(&self) -> String {
        "persistence-events".into()
    }

    fn source(&self) -> Arc<dyn StreamSource> {
        self.source.clone()
    }

    fn projector(&self) -> Arc<dyn Projector> {
        self.projector.clone()
    }

    fn prepare(&self, stream_name: &str, queue_message_id: &str, raw: &[u8]) -> NewInboxMessage {
        match model::decode(raw) {
            Ok((event, envelope)) => NewInboxMessage::valid(
                model::SOURCE,
                stream_name,
                queue_message_id,
                model::MESSAGE_TYPE,
                Some(event.schema_version as i32),
                envelope, // full original envelope, not just the payload
            ),
            Err(reason) => NewInboxMessage::malformed(
                model::SOURCE,
                stream_name,
                queue_message_id,
                raw.to_vec(),
                reason,
            ),
        }
    }
}
