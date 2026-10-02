use sqlx::{PgPool, QueryBuilder};

use crate::{
    error::ApiResult,
    dto::{
        user::{UserFilter, UserResponse},
        utils::{PageInfo, PaginatedList, Pagination},
    },
    model::user::{NewUser, User},
};

pub async fn insert_user(new_user: &NewUser, db_conn: &PgPool) -> ApiResult<User> {
    let res = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users(username, email, full_name, password_hash)
        VALUES($1, $2, $3, $4)
        RETURNING id, username, email, full_name,password_hash, created_at, updated_at
        "#,
        new_user.username,
        new_user.email,
        new_user.full_name,
        new_user.password_hash,
    )
    .fetch_one(db_conn)
    .await?;
    Ok(res)
}

pub async fn list(
    db_conn: &PgPool,
    query: &UserFilter,
    pagination: &Pagination,
) -> ApiResult<PaginatedList<UserResponse>> {
    // ---- build the WHERE clause once, reused for both count and data queries ----
    let build_where = |qb: &mut QueryBuilder<_>| {
        let mut has_condition = false;

        if let Some(username) = &query.username {
            qb.push(" WHERE username ILIKE ");
            qb.push_bind(format!("%{username}%"));
            has_condition = true;
        }

        if let Some(email) = &query.email {
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
    let total_rows: i64 = count_qb.build_query_scalar().fetch_one(db_conn).await?;

    // ---- actual page of data ----
    let mut data_qb: QueryBuilder<_> = QueryBuilder::new(
        "SELECT id, username, full_name, email, created_at, updated_at FROM users",
    );
    build_where(&mut data_qb);
    data_qb.push(" ORDER BY created_at DESC LIMIT ");
    data_qb.push_bind(pagination.get_page_size());
    data_qb.push(" OFFSET ");
    data_qb.push_bind(pagination.get_page_size() * (pagination.get_page_no() - 1));

    let rows = data_qb
        .build_query_as::<UserResponse>()
        .fetch_all(db_conn)
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
