use serde::{Deserialize, Serialize};

use crate::{prom::config::PromConfig, trace::config::TraceConfig};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MimirConfig {
    pub service_name: String,
    pub version: Option<String>,
    pub prom: PromConfig,
    pub trace: TraceConfig,
}
