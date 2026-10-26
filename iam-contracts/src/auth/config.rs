use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct AuthConfig {
    pub jwt_ttl_hours: u64,
}
