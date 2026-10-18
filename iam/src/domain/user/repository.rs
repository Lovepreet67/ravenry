use std::fmt::Debug;

use sqlx::{PgPool, QueryBuilder};

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync + Debug {
    async fn create(&self, user: &NewUser) -> ApiResult<User>;
    async fn list(
        &self,
        filters: &UserFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<User>>;
    async fn get(&self, id: &str) -> ApiResult<Option<User>>;
    async fn update(&self, user: &User) -> ApiResult<User>;
}
use crate::shared::{
    error::ApiResult,
    pagination::{PageInfo, PaginatedList, Pagination},
};

use crate::{
    domain::user::dto::UserFilter,
    domain::user::model::{NewUser, User},
};

#[derive(Debug)]
pub struct DatabaseUserRepository {
    db_conn: PgPool,
}
impl DatabaseUserRepository {
    pub fn new(db_conn: PgPool) -> Self {
        Self { db_conn }
    }
}

#[async_trait::async_trait]
impl UserRepository for DatabaseUserRepository {
    async fn create(&self, new_user: &NewUser) -> ApiResult<User> {
        let res = sqlx::query_as!(
            User,
            r#"
        INSERT INTO users(username, email, full_name, password_hash)
        VALUES($1, $2, $3, $4)
        RETURNING id, username, email, full_name,password_hash,is_active, created_at, updated_at
        "#,
            new_user.username,
            new_user.email,
            new_user.full_name,
            new_user.password_hash,
        )
        .fetch_one(&self.db_conn)
        .await?;
        Ok(res)
    }
    async fn list(
        &self,
        filter: &UserFilter,
        pagination: &Pagination,
    ) -> ApiResult<PaginatedList<User>> {
        // ---- build the WHERE clause once, reused for both count and data queries ----
        let build_where = |qb: &mut QueryBuilder<_>| {
            let mut has_condition = false;

            if let Some(username) = &filter.username {
                qb.push(" WHERE username ILIKE ");
                qb.push_bind(format!("%{username}%"));
                has_condition = true;
            }

            if let Some(email) = &filter.email {
                qb.push(if has_condition {
                    " AND email ILIKE "
                } else {
                    " WHERE email ILIKE "
                });
                qb.push_bind(format!("%{email}%"));
            }
        };

        // ---- total count, for page_info ----
        let mut count_qb: QueryBuilder<_> = QueryBuilder::new("SELECT COUNT(*) FROM users");
        build_where(&mut count_qb);
        let total_rows: i64 = count_qb
            .build_query_scalar()
            .fetch_one(&self.db_conn)
            .await?;

        // ---- actual page of data ----
        let mut data_qb: QueryBuilder<_> = QueryBuilder::new(
            "SELECT id, username, full_name, email,password_hash,is_active, created_at, updated_at FROM users",
        );
        build_where(&mut data_qb);
        data_qb.push(" ORDER BY created_at DESC LIMIT ");
        data_qb.push_bind(pagination.get_page_size());
        data_qb.push(" OFFSET ");
        data_qb.push_bind(pagination.get_page_size() * (pagination.get_page_no() - 1));

        let rows = data_qb
            .build_query_as::<User>()
            .fetch_all(&self.db_conn)
            .await?;

        Ok(PaginatedList {
            list: rows,
            page_info: PageInfo {
                page_size: pagination.get_page_size(),
                page_no: pagination.get_page_no(),
                total_rows: total_rows,
            },
        })
    }
    async fn get(&self, id: &str) -> ApiResult<Option<User>> {
        let id = uuid::Uuid::parse_str(id).map_err(|_| {
            crate::shared::error::ApiError::BadRequest("Invlid userId provided".into())
        })?;

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT * FROM users
            where id = $1
            "#,
            id,
        )
        .fetch_optional(&self.db_conn)
        .await?;
        Ok(user)
    }
    async fn update(&self, user: &User) -> ApiResult<User> {
        let updated = sqlx::query_as!(
            User,
            r#"
            UPDATE users 
            SET full_name = $1, is_active = $2
            WHERE id = $3
            RETURNING *
            "#,
            user.full_name,
            user.is_active,
            user.id
        )
        .fetch_optional(&self.db_conn)
        .await?
        .ok_or(crate::shared::error::ApiError::NotFound)?;
        Ok(updated)
    }
}
