use redis::{AsyncCommands, RedisError, aio::ConnectionManager};

pub async fn get_redis() -> Result<ConnectionManager, RedisError> {
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("Redis client working");
    ConnectionManager::new(client).await
}
pub async fn health_check() -> Result<(), RedisError> {
    get_redis().await?.ping().await
}

#[tokio::test]
async fn test_connection() {
    let mut conn = get_redis()
        .await
        .expect("Error while getting connection manager");
    println!("{:?}", conn);
    conn.ping::<()>().await.expect("Error while pingiing");
}
