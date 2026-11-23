use std::{collections::HashMap, time::Duration};

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_http::HeaderExtractor;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    Resource,
    propagation::TraceContextPropagator,
    trace::{Sampler, SdkTracerProvider},
};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::MimirConfig, trace::gaurd::TelemetryGuard};

pub mod config;
pub mod gaurd;

/// Call once, first thing in `main`. This owns the global `tracing`
/// subscriber, so remove any `tracing_subscriber::fmt().init()` from services.
///
/// A broken collector never stops a service from starting: if the exporter
/// can't be built, logs still work and tracing is simply off.
pub fn init(config: &MimirConfig) -> TelemetryGuard {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let cfg = &config.trace;
    let provider = if cfg.enabled {
        match build_provider(&config) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("tracing disabled, exporter failed to build: {e}");
                None
            }
        }
    } else {
        None
    };

    let otel_layer = provider.as_ref().map(|p| {
        tracing_opentelemetry::layer()
            .with_tracer(p.tracer(config.service_name.clone()))
            // Keep the exporter's own HTTP stack out of the trace stream,
            // otherwise exporting spans creates more spans.
            .with_filter(EnvFilter::new(
                "info,hyper=off,h2=off,reqwest=off,opentelemetry=off",
            ))
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")));

    // The otel layer must be added first so its subscriber type resolves to Registry.
    tracing_subscriber::registry()
        .with(otel_layer)
        .with(fmt_layer)
        .init();

    if let Some(p) = &provider {
        global::set_tracer_provider(p.clone());
    }
    TelemetryGuard { provider }
}

fn build_provider(cfg: &MimirConfig) -> Result<SdkTracerProvider, Box<dyn std::error::Error>> {
    let exporter = SpanExporter::builder()
        .with_http()
        // With programmatic config the signal path must be explicit.
        .with_endpoint(format!(
            "{}/v1/traces",
            cfg.trace.endpoint.trim_end_matches('/')
        ))
        .with_timeout(Duration::from_secs(3))
        .build()?;

    let resource = Resource::builder()
        .with_service_name(cfg.service_name.clone())
        .with_attributes([KeyValue::new(
            "service.version",
            cfg.version
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or_else(|| "1".into()),
        )])
        .build();

    Ok(SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            cfg.trace.sample_ratio,
        ))))
        .with_resource(resource)
        .build())
}

// ---------------------------------------------------------------------
// Propagation: keeps one trace across HTTP hops and Redis streams
// ---------------------------------------------------------------------

/// Current span's context as W3C headers (`traceparent`, `tracestate`).
/// Attach the map to an outgoing HTTP request or a stream message.
pub fn current_context_headers() -> HashMap<String, String> {
    let cx = tracing::Span::current().context();
    let mut carrier = HashMap::new();
    global::get_text_map_propagator(|p| p.inject_context(&cx, &mut carrier));
    carrier
}

/// Makes `span` a child of the context found in `carrier`. Call right after
/// creating a consumer span, before entering it.
pub fn set_parent_from(span: &tracing::Span, carrier: &HashMap<String, String>) {
    let cx = global::get_text_map_propagator(|p| p.extract(carrier));
    span.set_parent(cx);
}

/// Axum middleware: one server span per request, parented to the caller's
/// trace if a `traceparent` header is present.
/// Use as `.route_layer(axum::middleware::from_fn(mimir::trace::server_span))`.
pub async fn server_span(req: Request, next: Next) -> Response {
    // The template ("/tenants/{tenant_id}/events"), never the raw URL.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| "unmatched".into());
    let method = req.method().to_string();
    let name = format!("{method} {route}");

    let span = tracing::info_span!(
        "http.request",
        otel.name = %name,
        otel.kind = "server",
        http.request.method = %method,
        http.route = %route,
        http.response.status_code = tracing::field::Empty,
        otel.status_code = tracing::field::Empty,
    );

    let parent = global::get_text_map_propagator(|p| p.extract(&HeaderExtractor(req.headers())));
    span.set_parent(parent);

    let res = next.run(req).instrument(span.clone()).await;

    let status = res.status();
    span.record("http.response.status_code", status.as_u16());
    if status.is_server_error() {
        span.record("otel.status_code", "ERROR");
    }
    res
}
