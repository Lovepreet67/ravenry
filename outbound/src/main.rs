use std::sync::Arc;

use common::queue::{Queue, redis_stream::RedisQueue};

use crate::runner::Runner;

pub mod runner;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let redis_url = std::env::var("REDIS_URL")?;
    let client = redis::Client::open(redis_url)?;
    let conn = redis::aio::ConnectionManager::new(client).await?;
    let queue: Arc<dyn Queue> = Arc::new(RedisQueue::new(conn));
    let runner = Runner::new(queue);
    runner.start().await;
}
