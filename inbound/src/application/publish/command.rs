use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::application::publish::error::PublishError;

#[derive(Debug, Clone, Deserialize)]
pub struct PublishCommand {
    pub topic_path: String,
    pub destinations: Option<Vec<Uuid>>,
    pub payload: Value,
}

impl PublishCommand {
    pub fn validate(&self) -> Result<(), PublishError> {
        if let Some(destinations) = self.destinations.as_ref() {
            if destinations.len() == 0 {
                return Err( PublishError::InvalidRequest("Atleast One destination is required to publish event, sepecified zero destinations".into()));
            }
        }
        Ok(())
    }
}
