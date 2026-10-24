use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Deserialize, Debug, Serialize, Type, Clone, Copy)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "destination_kind", rename_all = "snake_case")]
pub enum DestinationType {
    Http,
}
