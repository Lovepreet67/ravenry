use axum::{Router, routing::get};

use crate::{
    controller::not_implemented,
    router::{auth::get_auth_router, tenant::get_tenant_router, user::get_user_router},
};
pub mod auth;
pub mod tenant;
pub mod user;

pub async fn get_router() -> Router {
    let router = Router::new();
    router
        .route("/", get(not_implemented))
        .nest_service("/tenant", get_tenant_router().await)
        .nest_service("/user", get_user_router().await)
        .nest_service("/auth", get_auth_router().await)
}
