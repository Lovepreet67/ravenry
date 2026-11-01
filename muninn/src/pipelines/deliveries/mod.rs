mod model;
mod projector;
mod source;

use std::sync::Arc;

use crate::core::inbox::NewInboxMessage;
use crate::core::{Pipeline, Projector, StreamSource};

pub struct DeliveriesPipeline {
    source: Arc<dyn StreamSource>,
    projector: Arc<dyn Projector>,
}

impl DeliveriesPipeline {
    pub fn new() -> Self {
        Self {
            source: Arc::new(source::source()),
            projector: Arc::new(projector::DeliveriesProjector),
        }
    }
}

impl Pipeline for DeliveriesPipeline {
    fn name(&self) -> &'static str {
        model::SOURCE
    }

    fn consumer_group(&self) -> String {
        "persistence-deliveries".into()
    }

    fn source(&self) -> Arc<dyn StreamSource> {
        self.source.clone()
    }

    fn projector(&self) -> Arc<dyn Projector> {
        self.projector.clone()
    }

    fn prepare(&self, stream_name: &str, queue_message_id: &str, raw: &[u8]) -> NewInboxMessage {
        match model::decode(raw) {
            Ok((o, envelope)) => NewInboxMessage::valid(
                model::SOURCE,
                stream_name,
                queue_message_id,
                model::MESSAGE_TYPE,
                Some(o.schema_version as i32),
                envelope,
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
