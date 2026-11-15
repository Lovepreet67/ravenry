use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct RetryCommand {
    pub delivery_id: Uuid,
    pub destination_id: Uuid,
}
