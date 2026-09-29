use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::{
    controller::tenant::{
        create,
        delete::delete as tenant_delete,
        list::{list, list_id},
        update::update,
    },
    state::AppState,
};

pub async fn get_tenant_router() -> Router {
    let state = AppState::new().await;
    Router::new()
        .route("/list", get(list))
        .route("/list/{tenant_id}", get(list_id))
        .route("/create", post(create::create))
        .route("/{tenant_id}", delete(tenant_delete).patch(update))
        .with_state(state)
}
