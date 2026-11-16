use axum::{
    Router,
    extract::{Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use metrics_exporter_prometheus::PrometheusHandle;
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;

use crate::config::MimirConfig;

#[derive(Clone)]
struct AppState {
    handle: PrometheusHandle,
    token: Option<String>,
}

async fn handler(State(app_state): State<AppState>) -> String {
    app_state.handle.render()
}

pub async fn serve(
    handle: PrometheusHandle,
    config: MimirConfig,
    shutdown: CancellationToken,
) -> std::io::Result<()> {
    let state = AppState {
        handle,
        token: config.bearer_token.clone(),
    };
    let app = Router::new()
        .route("/metrics", get(handler))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(config.addr.clone()).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move { shutdown.cancelled().await })
        .await
}

async fn auth(
    State(app_state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(expected) = &app_state.token {
        let given = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if !bool::from(given.as_bytes().ct_eq(expected.as_bytes())) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    Ok(next.run(req).await)
}
