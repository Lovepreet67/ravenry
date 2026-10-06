use core::panic;

pub mod controller;
pub mod error;
pub mod middleware;
pub mod router;
pub mod state;
use axum::Router;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::router::get_router;
pub mod dto;
pub mod model;
pub mod service;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    info!("Tracing is working");
    let app = Router::new()
        .merge(get_router().await)
        .layer(TraceLayer::new_for_http());

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3002").await {
        Ok(l) => l,
        Err(e) => {
            panic!("Error int tcp listener,  {:?}", e);
        }
    };
    _ = axum::serve(listener, app).await;
}
