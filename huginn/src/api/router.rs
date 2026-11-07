use axum::Router;
use tower_http::trace::TraceLayer;

use super::{deliveries, events, health};
use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(events::routes())
        .merge(deliveries::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
