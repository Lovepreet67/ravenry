use std::fmt::Debug;

use sqlx::PgPool;

use crate::{
    domain::membership::model::{MembershipDetails, UserMembership, UserMembershipDetails},
    shared::error::ApiResult,
};

#[async_trait::async_trait]
pub trait MembershipRepository: Send + Sync + Debug {
    async fn upsert(&self, membership: UserMembership) -> ApiResult<MembershipDetails>;
    async fn list(&self, user_id: uuid::Uuid) -> ApiResult<UserMembershipDetails>;
}

#[derive(Debug)]
pub struct DatabaseMembershipRepository {
    db_conn: PgPool,
}
impl DatabaseMembershipRepository {
    pub fn new(db_conn: PgPool) -> Self {
        Self { db_conn }
    }
}

#[async_trait::async_trait]
impl MembershipRepository for DatabaseMembershipRepository {
    async fn upsert(&self, membership: UserMembership) -> ApiResult<MembershipDetails> {
        let m = sqlx::query_as!(
            MembershipDetails,
            r#"
                WITH membership as (INSERT INTO memberships (user_id, tenant_id, role_id) 
                VALUES ($1, $2, $3)
                ON CONFLICT (user_id,tenant_id)
                DO UPDATE SET ROLE_ID = $3
                RETURNING user_id, tenant_id, role_id )
                SELECT 
                roles.title as role_title, roles.id as role_id,
                tenants.name as tenant_name, tenants.id as tenant_id
                FROM membership 
                INNER JOIN roles on roles.id = membership.role_id
                INNER JOIN tenants on tenants.id = membership.tenant_id
        "#,
            membership.user_id,
            membership.tenant_id,
            membership.role_id,
        )
        .fetch_one(&self.db_conn)
        .await?;
        Ok(m)
    }
    async fn list(&self, user_id: uuid::Uuid) -> ApiResult<UserMembershipDetails> {
        let memberships = sqlx::query_as!(
            MembershipDetails,
            r#"
        SELECT 
        roles.title as role_title, roles.id as role_id,
        tenants.name as tenant_name, tenants.id as tenant_id
        FROM memberships 
        INNER JOIN roles on roles.id = memberships.role_id
        INNER JOIN tenants on tenants.id = memberships.tenant_id
        where memberships.user_id = $1
        "#,
            user_id
        )
        .fetch_all(&self.db_conn)
        .await?;
        return Ok(UserMembershipDetails {
            user_id: user_id,
            memberships,
        });
    }
}
