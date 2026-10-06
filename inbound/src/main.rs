use std::{println, sync::Arc};

use common::queue::{redis_stream::RedisQueue, Queue};

mod http;
pub mod model;

#[derive(Clone)]
pub struct AppState {
    pub queue: Arc<dyn Queue>,
    pub jwt_secret: Arc<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let redis_url = std::env::var("REDIS_URL")?;
    let jwt_secret = std::env::var("JWT_SECRET")?;
    println!("Both found");

    let client = redis::Client::open(redis_url)?;
    let conn = redis::aio::ConnectionManager::new(client).await?;
    let queue: Arc<dyn Queue> = Arc::new(RedisQueue::new(conn));

    let state = AppState {
        queue,
        jwt_secret: Arc::new(jwt_secret),
    };

    let app = http::router::build(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("inbound listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
