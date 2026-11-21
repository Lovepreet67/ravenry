use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PromConfig {
    pub addr: String,
    pub bearer_token: Option<String>,
}
impl Default for PromConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:9091".into(),
            bearer_token: None,
        }
    }
}
