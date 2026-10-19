use std::fmt::Debug;

use sqlx::PgPool;

use crate::{domain::role::model, shared::error::ApiResult};

#[async_trait::async_trait]
pub trait RoleRepository: Send + Sync + Debug {
    async fn list(&self) -> ApiResult<model::Roles>;
    async fn get_by_slug(&self, slug: &str) -> ApiResult<Option<model::Role>>;
}

#[derive(Debug)]
pub struct DatabaseRoleRepository {
    db_conn: PgPool,
}
impl DatabaseRoleRepository {
    pub fn new(db_conn: PgPool) -> Self {
        Self { db_conn }
    }
}

#[async_trait::async_trait]
impl RoleRepository for DatabaseRoleRepository {
    async fn list(&self) -> ApiResult<model::Roles> {
        let res = sqlx::query_as!(
            model::Role,
            r#"
            SELECT * FROM ROLES; 
            "#
        )
        .fetch_all(&self.db_conn)
        .await?;
        Ok(res)
    }
    async fn get_by_slug(&self, slug: &str) -> ApiResult<Option<model::Role>> {
        let res = sqlx::query_as!(
            model::Role,
            r#"
            SELECT * FROM ROLES where slug = $1
            "#,
            slug
        )
        .fetch_optional(&self.db_conn)
        .await?;
        Ok(res)
    }
}
