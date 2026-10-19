use axum::{Router, routing::get};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/tenants/{tenant_id}/destinations",
            get(not_implemented).post(not_implemented),
        )
        .route(
            "/tenants/{tenant_id}/destinations/{destination_id}",
            get(not_implemented)
                .patch(not_implemented)
                .delete(not_implemented),
        )
}

async fn not_implemented() -> &'static str {
    "Not Implemented"
}
