use serde::{Deserialize, Deserializer};
use uuid::Uuid;

fn deserialize_timeout<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;

    value.parse::<u64>().map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Deserialize)]
pub struct DestinationProjection {
    pub name: String,
    pub url: String,
    pub kind: String,
    pub status: String,
    #[serde(deserialize_with = "deserialize_timeout")]
    pub timeout: u64,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedDestination {
    pub id: Uuid,
    pub config: DestinationProjection,
}
