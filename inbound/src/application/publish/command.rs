use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct PublishCommand {
    pub topic_path: String,
    pub destinations: Vec<Uuid>,
    pub payload: Value,
}
