use std::env;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::error::ApiResult;
pub async fn get_db() -> ApiResult<PgPool> {
    let db_url = env::var("DATABASE_URL")?;
    let res = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await?;
    Ok(res)
}

#[tokio::test]
async fn test_db_connection() {
    let conn = get_db().await.expect("Error getting the conneciton pool");
    sqlx::query!("Select * from USER").fetch_all(&conn).await;
}
