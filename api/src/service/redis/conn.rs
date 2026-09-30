use std::env;

use redis::{AsyncTypedCommands, aio::ConnectionManager};

use crate::Error::ApiResult;

pub async fn get_redis() -> ApiResult<ConnectionManager> {
    let redis_url = env::var("REDIS_URL")?;
    let client = redis::Client::open(redis_url).expect("Redis client working");
    let res = ConnectionManager::new(client).await?;
    Ok(res)
}
pub async fn health_check() -> ApiResult<()> {
    get_redis().await?.ping().await?;
    Ok(())
}

#[tokio::test]
async fn test_connection() {
    let mut conn = get_redis()
        .await
        .expect("Error while getting connection manager");
    println!("{:?}", conn);
    conn.ping().await.expect("Error while pingiing");
}
