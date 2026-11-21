pub mod config;
pub mod prom;
pub mod trace;
use std::pin::Pin;

pub use prom::*;
use tokio_util::sync::CancellationToken;

use crate::{config::MimirConfig, trace::gaurd::TelemetryGuard};

pub struct MimirGaurd {
    pub prom: Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + Send>>,
    pub telemetry: TelemetryGuard,
}

pub fn init(config: MimirConfig, cancellation_token: CancellationToken) -> MimirGaurd {
    let prom_handle = prom::init(&config.service_name);
    let y = trace::init(&config);
    let x = Box::pin(prom::server::serve(
        prom_handle,
        config.prom.clone(),
        cancellation_token,
    ));
    MimirGaurd {
        prom: x,
        telemetry: y,
    }
}
