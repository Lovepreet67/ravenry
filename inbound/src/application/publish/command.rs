use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct PublishCommand {
    pub topic_path: String,
    pub payload: Value,
}
