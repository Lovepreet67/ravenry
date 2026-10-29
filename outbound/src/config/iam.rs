use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IamConfig {
    pub public_key_url: String,
    pub issuer: String,
    pub audience: String,
    pub publish_roles: Vec<String>,
}
