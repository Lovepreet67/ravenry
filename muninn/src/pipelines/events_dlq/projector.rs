use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use iam_contracts::events::PublishedEventDlq;
use iam_contracts::streams::topic_events_dlq;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::inbox::InboxMessage;
use crate::core::{ProjectError, Projector};

pub struct EventsDlqProjector;

/// Shared by the single and bulk paths: decode + tenant/stream isolation check.
fn decode_and_validate(msg: &InboxMessage) -> Result<PublishedEventDlq, ProjectError> {
    let payload = msg
        .payload
        .as_ref()
        .ok_or_else(|| ProjectError::Invalid("inbox row has no payload".into()))?;

    let event: PublishedEventDlq = serde_json::from_value(payload.clone())?;

    let expected = topic_events_dlq(event.tenant_id, event.topic_id);
    if msg.stream_name != expected {
        return Err(ProjectError::Invalid(format!(
            "event dlq {} claims tenant/topic that do not match stream {}",
            event.event_id, msg.stream_name
        )));
    }
    Ok(event)
}

/// The row already exists under a different tenant/topic, so we refuse to
/// touch it. Same class of problem as a forged message.
fn owner_mismatch(event_id: Uuid) -> ProjectError {
    ProjectError::Invalid(format!(
        "event {event_id} already exists under a different tenant/topic"
    ))
}

#[async_trait]
impl Projector for EventsDlqProjector {
    async fn project(
        &self,
        conn: &mut PgConnection,
        msg: &InboxMessage,
    ) -> Result<(), ProjectError> {
        let event = decode_and_validate(msg)?;

        // Upsert. The WHERE on DO UPDATE keeps tenant isolation: if the
        // existing row belongs to another tenant/topic, nothing is updated.
        let res = sqlx::query(
            r#"
            INSERT INTO events
                (event_id, tenant_id, topic_id, topic_path, schema_version, raw_payload, error)
            VALUES ($1, $2, $3, '', $4, $5, $6)
            ON CONFLICT (event_id) DO UPDATE
               SET error       = EXCLUDED.error,
                   raw_payload = COALESCE(EXCLUDED.raw_payload, events.raw_payload)
             WHERE events.tenant_id = EXCLUDED.tenant_id
               AND events.topic_id  = EXCLUDED.topic_id
            "#,
        )
        .bind(event.event_id)
        .bind(event.tenant_id)
        .bind(event.topic_id)
        .bind(i32::from(event.schema_version))
        .bind(&event.raw_payload)
        .bind(&event.error)
        .execute(&mut *conn)
        .await?;

        if res.rows_affected() == 0 {
            return Err(owner_mismatch(event.event_id));
        }
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
        let mut results: Vec<Result<(), ProjectError>> = Vec::with_capacity(msgs.len());
        let mut valid: Vec<(usize, PublishedEventDlq)> = Vec::with_capacity(msgs.len());

        for (i, msg) in msgs.iter().enumerate() {
            match decode_and_validate(msg) {
                Ok(ev) => {
                    results.push(Ok(())); // provisional, fixed up below
                    valid.push((i, ev));
                }
                Err(e) => results.push(Err(e)),
            }
        }

        if valid.is_empty() {
            return Ok(results);
        }

        // DO UPDATE fails with "cannot affect row a second time" if one
        // event_id appears twice in a batch, so dedupe (last wins).
        let mut latest: HashMap<Uuid, &PublishedEventDlq> = HashMap::new();
        for (_, ev) in &valid {
            latest.insert(ev.event_id, ev);
        }

        let n = latest.len();
        let mut event_ids = Vec::with_capacity(n);
        let mut tenant_ids = Vec::with_capacity(n);
        let mut topic_ids = Vec::with_capacity(n);
        let mut versions: Vec<i32> = Vec::with_capacity(n);
        let mut raw_payloads: Vec<Option<String>> = Vec::with_capacity(n);
        let mut errors = Vec::with_capacity(n);
        for ev in latest.values() {
            event_ids.push(ev.event_id);
            tenant_ids.push(ev.tenant_id);
            topic_ids.push(ev.topic_id);
            versions.push(i32::from(ev.schema_version));
            raw_payloads.push(ev.raw_payload.clone());
            errors.push(ev.error.clone());
        }

        // RETURNING only yields rows that were inserted or updated. A row
        // blocked by the tenant/topic WHERE is absent from the result.
        let written: HashSet<Uuid> = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO events
                (event_id, tenant_id, topic_id, topic_path, schema_version, raw_payload, error)
            SELECT event_id, tenant_id, topic_id, '', schema_version, raw_payload, error
              FROM UNNEST(
                    $1::uuid[], $2::uuid[], $3::uuid[],
                    $4::int4[], $5::text[], $6::text[]
                   ) AS u(event_id, tenant_id, topic_id, schema_version, raw_payload, error)
            ON CONFLICT (event_id) DO UPDATE
               SET error       = EXCLUDED.error,
                   raw_payload = COALESCE(EXCLUDED.raw_payload, events.raw_payload)
             WHERE events.tenant_id = EXCLUDED.tenant_id
               AND events.topic_id  = EXCLUDED.topic_id
            RETURNING event_id
            "#,
        )
        .bind(&event_ids)
        .bind(&tenant_ids)
        .bind(&topic_ids)
        .bind(&versions)
        .bind(&raw_payloads)
        .bind(&errors)
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();

        for (i, ev) in &valid {
            if !written.contains(&ev.event_id) {
                results[*i] = Err(owner_mismatch(ev.event_id));
            }
        }

        Ok(results)
    }
}
