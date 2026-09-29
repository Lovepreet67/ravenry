use sqlx::{PgPool, postgres::PgPoolOptions};
const DB_URL: &str = "postgress://postgres:root@localhost:5432/tenant_db";
pub async fn get_db() -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(DB_URL)
        .await
}

#[tokio::test]
async fn test_db_connection() {
    let conn = get_db().await.expect("Error getting the conneciton pool");
    sqlx::query!("Select * from USER").fetch_all(&conn).await;
}
