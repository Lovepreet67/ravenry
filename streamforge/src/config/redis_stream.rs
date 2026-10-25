use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RedisStreamConfig {
    pub url: String,
}
