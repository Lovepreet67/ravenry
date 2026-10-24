use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    auth::handler::router as auth_router,
    domain::{
        destination::handler::router as destination_router,
        membership::handler::router as membership_router, role::handler::router as role_router,
        tenant::handler::router as tenant_router, topic::handler::router as topic_router,
        user::handler::router as user_router,
    },
    state::AppState,
};
pub fn router() -> Router<AppState> {
    let cors = CorsLayer::permissive();
    let tr: Router<AppState> = tenant_router();
    let ur: Router<AppState> = user_router();
    let mr: Router<AppState> = membership_router();
    let ar: Router<AppState> = auth_router();
    let rr: Router<AppState> = role_router();
    let topic_r: Router<AppState> = destination_router();
    let dr: Router<AppState> = topic_router();
    Router::new()
        .merge(tr)
        .merge(ur)
        .merge(mr)
        .merge(ar)
        .merge(dr)
        .merge(topic_r)
        .nest("/roles", rr)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
}
