use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    pub environment: String,
    pub endpoint: String,
    pub sample_ratio: f64,
    pub enabled: bool,
}
