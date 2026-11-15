use async_trait::async_trait;
use iam_contracts::events::RetryEvent;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::inbox::InboxMessage;
use crate::core::{ProjectError, Projector};

pub struct RetryProjector;

#[async_trait]
impl Projector for RetryProjector {
    async fn project(
        &self,
        conn: &mut PgConnection,
        msg: &InboxMessage,
    ) -> Result<(), ProjectError> {
        let payload = msg
            .payload
            .as_ref()
            .ok_or_else(|| ProjectError::Invalid("inbox row has no payload".into()))?;

        let o: RetryEvent = serde_json::from_value(payload.clone())?;

        // 1. Delivery must exist
        let event: Option<(Uuid, i32, Uuid)> = sqlx::query_as(
            "SELECT tenant_id,current_attempt, destination_id FROM deliveries 
            WHERE delivery_id = $1 ",
        )
        .bind(o.delivery_id)
        .fetch_optional(&mut *conn)
        .await?;

        match event {
            None => {
                return Err(ProjectError::Defer(format!(
                    "delivery {} not persisted yet",
                    o.delivery_id
                )))
            }
            // If delivery outcome has alredy updated the request this event can be dropped
            Some((_, curr_attempt, _)) if curr_attempt != o.curr_attempt => return Ok(()),
            // Tenant isolation: the outcome must agree with its event.
            Some((tenant_id, _, destination_id))
                if tenant_id != o.tenant_id || destination_id != o.destination_id =>
            {
                return Err(ProjectError::Invalid(format!(
                    "outcome tenant/destinantion do not match event {}",
                    o.destination_id
                )))
            }
            Some(_) => {}
        }

        // 2. Find or create the logical delivery. A retry reuses the row.
        sqlx::query(
            r#"
            UPDATE deliveries SET status=$1
            WHERE delivery_id = $2
            "#,
        )
        .bind("pending")
        .bind(o.delivery_id)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}
