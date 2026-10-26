use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectionConfig {
    pub redis_url: String,
}
