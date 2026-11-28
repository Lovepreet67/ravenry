use async_trait::async_trait;
use iam_contracts::events::PublishedEvent;
use iam_contracts::streams::topic_events;
use sqlx::PgConnection;

use crate::core::inbox::InboxMessage;
use crate::core::{ProjectError, Projector};

pub struct EventsProjector;

/// Shared by single and bulk paths: decode + tenant/stream isolation check.
fn decode_and_validate(msg: &InboxMessage) -> Result<PublishedEvent, ProjectError> {
    let payload = msg
        .payload
        .as_ref()
        .ok_or_else(|| ProjectError::Invalid("inbox row has no payload".into()))?;

    // A deserialize failure maps to Invalid via From<serde_json::Error>.
    let event: PublishedEvent = serde_json::from_value(payload.clone())?;

    // Tenant isolation: a mismatch means a misrouted or forged message,
    // so it is quarantined, not stored.
    let expected = topic_events(event.tenant_id, event.topic_id);
    if msg.stream_name != expected {
        return Err(ProjectError::Invalid(format!(
            "event {} claims tenant/topic that do not match stream {}",
            event.event_id, msg.stream_name
        )));
    }
    Ok(event)
}

#[async_trait]
impl Projector for EventsProjector {
    async fn project(
        &self,
        conn: &mut PgConnection,
        msg: &InboxMessage,
    ) -> Result<(), ProjectError> {
        let event = decode_and_validate(msg)?;

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

    async fn bulk_project_supported(&self) -> bool {
        true
    }

    async fn project_bulk(
        &self,
        conn: &mut PgConnection,
        msgs: &[InboxMessage],
    ) -> anyhow::Result<Vec<Result<(), ProjectError>>> {
        // One result per input message, index-aligned with `msgs`.
        let mut results: Vec<Result<(), ProjectError>> = Vec::with_capacity(msgs.len());

        let mut event_ids = Vec::with_capacity(msgs.len());
        let mut tenant_ids = Vec::with_capacity(msgs.len());
        let mut topic_ids = Vec::with_capacity(msgs.len());
        let mut topic_paths = Vec::with_capacity(msgs.len());
        let mut versions: Vec<i32> = Vec::with_capacity(msgs.len());
        let mut payloads = Vec::with_capacity(msgs.len());

        for msg in msgs {
            match decode_and_validate(msg) {
                Ok(ev) => {
                    event_ids.push(ev.event_id);
                    tenant_ids.push(ev.tenant_id);
                    topic_ids.push(ev.topic_id);
                    topic_paths.push(ev.topic_path);
                    versions.push(ev.schema_version as i32);
                    payloads.push(ev.payload);
                    results.push(Ok(()));
                }
                // Bad rows get their own Err; they never poison the batch.
                Err(e) => results.push(Err(e)),
            }
        }

        if event_ids.is_empty() {
            return Ok(results);
        }

        // Single round trip. DO NOTHING is safe even when the same event_id
        // appears twice in one batch (only DO UPDATE has that restriction).
        sqlx::query(
            r#"
            INSERT INTO events
                (event_id, tenant_id, topic_id, topic_path, schema_version, payload)
            SELECT * FROM UNNEST(
                $1::uuid[], $2::uuid[], $3::uuid[], $4::text[], $5::int4[], $6::jsonb[]
            )
            ON CONFLICT (event_id) DO NOTHING
            "#,
        )
        .bind(&event_ids)
        .bind(&tenant_ids)
        .bind(&topic_ids)
        .bind(&topic_paths)
        .bind(&versions)
        .bind(&payloads)
        .execute(&mut *conn)
        .await?;

        Ok(results)
    }
}
