use redis::aio::ConnectionManager;

pub async fn connect(redis_url: &str) -> anyhow::Result<ConnectionManager> {
    let client = redis::Client::open(redis_url)?;
    let connection = ConnectionManager::new(client).await?;

    Ok(connection)
}
