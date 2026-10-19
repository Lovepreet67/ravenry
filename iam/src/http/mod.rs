use axum::Router;
use tower_http::trace::TraceLayer;

use crate::{
    auth::handler::router as auth_router,
    domain::{
        membership::handler::router as membership_router, role::handler::router as role_router,
        tenant::handler::router as tenant_router, user::handler::router as user_router,
    },
    state::AppState,
};
pub fn router() -> Router<AppState> {
    let tr: Router<AppState> = tenant_router();
    let ur: Router<AppState> = user_router();
    let mr: Router<AppState> = membership_router();
    let ar: Router<AppState> = auth_router();
    let rr: Router<AppState> = role_router();
    Router::new()
        .merge(tr)
        .merge(ur)
        .merge(mr)
        .merge(ar)
        .nest("/roles", rr)
        .layer(TraceLayer::new_for_http())
}
