use async_trait::async_trait;
use iam_contracts::delivery_outcome::DeliveryOutcome; // ADJUST
use sqlx::PgConnection;
use uuid::Uuid;

use super::model;
use crate::core::inbox::InboxMessage;
use crate::core::{ProjectError, Projector};

pub struct DeliveriesProjector;

#[async_trait]
impl Projector for DeliveriesProjector {
    async fn project(
        &self,
        conn: &mut PgConnection,
        msg: &InboxMessage,
    ) -> Result<(), ProjectError> {
        let payload = msg
            .payload
            .as_ref()
            .ok_or_else(|| ProjectError::Invalid("inbox row has no payload".into()))?;

        let o: DeliveryOutcome = serde_json::from_value(payload.clone())?;
        model::validate(&o).map_err(ProjectError::Invalid)?;

        // 1. The event must already be stored. Events and outcomes are
        //    consumed independently, so it may simply not have arrived yet.
        //    This is an explicit check rather than relying on the FK error.
        let event: Option<(Uuid, Uuid)> =
            sqlx::query_as("SELECT tenant_id, topic_id FROM events WHERE event_id = $1")
                .bind(o.event_id)
                .fetch_optional(&mut *conn)
                .await?;

        match event {
            None => {
                return Err(ProjectError::Defer(format!(
                    "event {} not persisted yet",
                    o.event_id
                )))
            }
            // Tenant isolation: the outcome must agree with its event.
            Some((tenant, topic)) if tenant != o.tenant_id || topic != o.topic_id => {
                return Err(ProjectError::Invalid(format!(
                    "outcome tenant/topic do not match event {}",
                    o.event_id
                )))
            }
            Some(_) => {}
        }

        // 2. Find or create the logical delivery. A retry reuses the row.
        sqlx::query(
            r#"
            INSERT INTO deliveries
                (delivery_id, tenant_id, event_id, topic_id, destination_id,
                 status, current_attempt)
            VALUES ($1, $2, $3, $4, $5, 'pending', 0)
            ON CONFLICT (delivery_id) DO NOTHING
            "#,
        )
        .bind(o.delivery_id)
        .bind(o.tenant_id)
        .bind(o.event_id)
        .bind(o.topic_id)
        .bind(o.destination_id)
        .execute(&mut *conn)
        .await?;

        // The delivery may predate this outcome. Its identity must match.
        let (d_tenant, d_event, d_dest): (Uuid, Uuid, Uuid) = sqlx::query_as(
            "SELECT tenant_id, event_id, destination_id FROM deliveries WHERE delivery_id = $1",
        )
        .bind(o.delivery_id)
        .fetch_one(&mut *conn)
        .await?;

        if d_tenant != o.tenant_id || d_event != o.event_id || d_dest != o.destination_id {
            return Err(ProjectError::Invalid(format!(
                "delivery {} identity conflicts with stored delivery",
                o.delivery_id
            )));
        }

        // 3. Immutable attempt. A replay of the same attempt_id is a no-op.
        //    The same (delivery_id, attempt_number) under a DIFFERENT
        //    attempt_id raises a unique violation, which is a data error.
        sqlx::query(
            r#"
            INSERT INTO delivery_attempts
                (attempt_id, delivery_id, attempt_number, success,
                 status_code, error, attempted_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (attempt_id) DO NOTHING
            "#,
        )
        .bind(o.attempt_id)
        .bind(o.delivery_id)
        .bind(o.attempt as i32)
        .bind(o.success)
        .bind(o.status_code.map(|c| c as i32))
        .bind(&o.error)
        .bind(o.attempted_at)
        .execute(&mut *conn)
        .await
        .map_err(|e| match &e {
            sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
                ProjectError::Invalid(format!(
                    "attempt {} of delivery {} already recorded under another attempt_id",
                    o.attempt, o.delivery_id
                ))
            }
            _ => ProjectError::from(e),
        })?;

        // 4. Monotonic state update. A late attempt 2 cannot overwrite
        //    the state set by attempt 3. Safe to run on replays too.
        sqlx::query(
            r#"
            UPDATE deliveries
               SET status = $1, current_attempt = $2, updated_at = NOW()
             WHERE delivery_id = $3 AND current_attempt < $2
            "#,
        )
        .bind(model::status_for(&o))
        .bind(o.attempt as i32)
        .bind(o.delivery_id)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::PgPool;

    use super::*;

    fn outcome(
        delivery: Uuid,
        event: Uuid,
        tenant: Uuid,
        topic: Uuid,
        attempt: u32,
        success: bool,
        will_retry: bool,
    ) -> DeliveryOutcome {
        DeliveryOutcome {
            schema_version: 1,
            delivery_id: delivery,
            attempt_id: Uuid::new_v4(),
            attempt,
            event_id: event,
            tenant_id: tenant,
            topic_id: topic,
            destination_id: Uuid::nil(),
            success,
            will_retry,
            status_code: Some(if success { 200 } else { 500 }),
            error: None,
            attempted_at: Utc::now(),
        }
    }

    fn inbox(o: &DeliveryOutcome) -> InboxMessage {
        InboxMessage {
            id: 1,
            source: "deliveries".into(),
            stream_name: "s".into(),
            queue_message_id: "1-0".into(),
            message_type: "delivery_outcome".into(),
            schema_version: Some(1),
            payload: Some(serde_json::to_value(o).unwrap()),
            attempts: 0,
            deferrals: 0,
            received_at: Utc::now(),
        }
    }

    async fn insert_event(pool: &PgPool, event: Uuid, tenant: Uuid, topic: Uuid) {
        sqlx::query(
            "INSERT INTO events (event_id, tenant_id, topic_id, topic_path, schema_version, payload)
             VALUES ($1, $2, $3, 't', 1, '{}')",
        )
        .bind(event).bind(tenant).bind(topic)
        .execute(pool).await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn outcome_before_event_is_deferred(pool: PgPool) {
        let o = outcome(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            1,
            true,
            false,
        );
        let mut conn = pool.acquire().await.unwrap();
        let res = DeliveriesProjector.project(&mut conn, &inbox(&o)).await;
        assert!(matches!(res, Err(ProjectError::Defer(_))));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn retry_history_is_kept_and_replays_are_noops(pool: PgPool) {
        let (d, e, t, tp) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        insert_event(&pool, e, t, tp).await;
        let mut conn = pool.acquire().await.unwrap();

        let a1 = outcome(d, e, t, tp, 1, false, true);
        let a2 = outcome(d, e, t, tp, 2, true, false);
        for o in [&a1, &a2, &a2] {
            // the second a2 is a duplicate delivery of the same outcome
            DeliveriesProjector
                .project(&mut conn, &inbox(o))
                .await
                .unwrap();
        }

        let (n,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM delivery_attempts WHERE delivery_id=$1")
                .bind(d)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(n, 2);

        let (status, cur): (String, i32) =
            sqlx::query_as("SELECT status, current_attempt FROM deliveries WHERE delivery_id=$1")
                .bind(d)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!((status.as_str(), cur), ("success", 2));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn late_older_attempt_does_not_regress_state(pool: PgPool) {
        let (d, e, t, tp) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        insert_event(&pool, e, t, tp).await;
        let mut conn = pool.acquire().await.unwrap();

        // attempt 3 (success) is processed before attempt 2 (failure)
        DeliveriesProjector
            .project(&mut conn, &inbox(&outcome(d, e, t, tp, 3, true, false)))
            .await
            .unwrap();
        DeliveriesProjector
            .project(&mut conn, &inbox(&outcome(d, e, t, tp, 2, false, true)))
            .await
            .unwrap();

        let (status, cur): (String, i32) =
            sqlx::query_as("SELECT status, current_attempt FROM deliveries WHERE delivery_id=$1")
                .bind(d)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!((status.as_str(), cur), ("success", 3));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn tenant_mismatch_is_quarantined(pool: PgPool) {
        let (d, e, t, tp) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        insert_event(&pool, e, t, tp).await;
        let mut conn = pool.acquire().await.unwrap();

        let forged = outcome(d, e, Uuid::new_v4(), tp, 1, true, false);
        let res = DeliveriesProjector
            .project(&mut conn, &inbox(&forged))
            .await;
        assert!(matches!(res, Err(ProjectError::Invalid(_))));
    }
}
