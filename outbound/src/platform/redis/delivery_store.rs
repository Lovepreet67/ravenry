use chrono::{DateTime, Utc};
use iam_contracts::streams::DELIVERY_RESULTS;

use crate::application::delivery::{dlq::DlqMessage, outcome::DeliveryOutcome};

const COMPLETE_ATTEMPT: &str = include_str!("scripts/complete_attempt.lua");

impl DeliveryStore {
    pub async fn complete_attempt(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        outcome: &DeliveryOutcome,
        next_attempt_at: Option<DateTime<Utc>>,
        request: &serde_json::Value,
    ) -> anyhow::Result<CompleteAttemptResult> {
        let mut connection = self.connection.clone();

        let state_key = keys::delivery_state(tenant_id, destination_id, outcome.delivery_id);

        let retry_key = keys::destination_retry(tenant_id, destination_id);

        let dlq_pending_key = keys::destination_dlq_pending(tenant_id, destination_id);

        let outcome_json = serde_json::to_string(outcome)?;

        let dlq_json = if !outcome.success && !outcome.will_retry {
            serde_json::to_string(&DlqMessage {
                outcome: outcome.clone(),
                request: request.clone(),
            })?
        } else {
            String::new()
        };

        let next_attempt_ms = next_attempt_at
            .map(|time| time.timestamp_millis())
            .unwrap_or(0);

        let script = Script::new(COMPLETE_ATTEMPT);

        let result: i64 = script
            .key(state_key)
            .key(retry_key)
            .key(DELIVERY_RESULTS)
            .key(dlq_pending_key)
            .arg(outcome.attempt_id.to_string())
            .arg(outcome_json)
            .arg(if outcome.success { 1 } else { 0 })
            .arg(if outcome.will_retry { 1 } else { 0 })
            .arg(outcome.delivery_id.to_string())
            .arg(next_attempt_ms)
            .arg(dlq_json)
            .invoke_async(&mut connection)
            .await
            .context("failed to complete delivery attempt")?;

        match result {
            1 => Ok(CompleteAttemptResult::Completed),
            -1 => Ok(CompleteAttemptResult::Missing),
            -2 => Ok(CompleteAttemptResult::NotProcessing),
            -3 => Ok(CompleteAttemptResult::StaleAttempt),
            other => anyhow::bail!("unexpected complete_attempt result: {other}"),
        }
    }
}
