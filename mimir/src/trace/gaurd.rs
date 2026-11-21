use opentelemetry_sdk::trace::SdkTracerProvider;

pub struct TelemetryGuard {
    pub(super) provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(p) = self.provider.take() {
            if let Err(e) = p.shutdown() {
                eprintln!("trace shutdown failed: {e}");
            }
        }
    }
}
