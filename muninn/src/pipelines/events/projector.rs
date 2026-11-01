use async_trait::async_trait;
use iam_contracts::events::PublishedEvent; // ADJUST
use iam_contracts::streams::topic_events;  // ADJUST
use sqlx::PgConnection;

use crate::core::inbox::InboxMessage;
use crate::core::{ProjectError, Projector};

pub struct EventsProjector;

#[async_trait]
impl Projector for EventsProjector {
    async fn project(
        &self,
        conn: &mut PgConnection,
        msg: &InboxMessage,
    ) -> Result<(), ProjectError> {
        let payload = msg
            .payload
            .as_ref()
            .ok_or_else(|| ProjectError::Invalid("inbox row has no payload".into()))?;

        // A deserialize failure maps to Invalid via From<serde_json::Error>.
        let event: PublishedEvent = serde_json::from_value(payload.clone())?;

        // Tenant isolation: the event must have come from the stream that
        // belongs to its own tenant and topic. A mismatch means a
        // misrouted or forged message, so it is quarantined, not stored.
        let expected = topic_events(event.tenant_id, event.topic_id);
        if msg.stream_name != expected {
            return Err(ProjectError::Invalid(format!(
                "event {} claims tenant/topic that do not match stream {}",
                event.event_id, msg.stream_name
            )));
        }

        // Idempotent by event_id. A replay is a silent no-op.
        sqlx::query(
            r#"
            INSERT INTO events
                (event_id, tenant_id, topic_id, topic_path, schema_version, payload)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(event.event_id)
        .bind(event.tenant_id)
        .bind(event.topic_id)
        .bind(&event.topic_path)
        .bind(event.schema_version as i32)
        .bind(&event.payload)
        .execute(&mut *conn)
        .await?;

        Ok(())
    }
}