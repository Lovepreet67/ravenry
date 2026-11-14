use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

fn deserialize_num_or_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + Deserialize<'de>,
    T::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrString<T> {
        Num(T),
        Str(String),
    }

    match NumOrString::<T>::deserialize(deserializer)? {
        NumOrString::Num(val) => Ok(val),
        NumOrString::Str(s) => s.parse::<T>().map_err(serde::de::Error::custom),
    }
}

fn default_max_attempts() -> u32 {
    3
}
fn default_retry_delay_ms() -> u64 {
    200
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DestinationProjection {
    pub name: String,
    pub url: String,
    pub kind: String,
    pub status: String,
    #[serde(deserialize_with = "deserialize_num_or_string")]
    pub timeout: u64,
    #[serde(
        default = "default_max_attempts",
        deserialize_with = "deserialize_num_or_string"
    )]
    pub max_attempts: u32,
    #[serde(
        default = "default_retry_delay_ms",
        deserialize_with = "deserialize_num_or_string"
    )]
    pub retry_delay_ms: u64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDestination {
    pub id: Uuid,
    pub config: DestinationProjection,
}
