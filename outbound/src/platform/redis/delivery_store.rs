use std::time::Duration;

use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use iam_contracts::{delivery_outcome::DeliveryOutcome, keys, streams::DELIVERY_RESULTS};
use redis::{aio::ConnectionManager, AsyncCommands, Script};
use uuid::Uuid;

use crate::application::delivery::model::{
    AcquireResult, CompleteAttemptResult, DeliveryRequest, DlqMessage,
};

const REGISTER_DELIVERY: &str = include_str!("scripts/register_delivery.lua");

const ACQUIRE_DELIVERY: &str = include_str!("scripts/acquire_delivery.lua");

const CLAIM_RETRY: &str = include_str!("scripts/claim_retry.lua");

const COMPLETE_ATTEMPT: &str = include_str!("scripts/complete_attempt.lua");
const ACK_DLQ_MESSAGE: &str = include_str!("scripts/ack_dlq_message.lua");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PendingDlqDestination {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

#[derive(Clone)]
pub struct DeliveryStore {
    connection: ConnectionManager,
}

impl DeliveryStore {
    pub async fn connect(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url).context("failed to create Redis client")?;

        let connection = ConnectionManager::new(client)
            .await
            .context("failed to connect to Redis")?;

        Ok(Self { connection })
    }

    pub async fn health_check(&self) -> anyhow::Result<()> {
        let mut connection = self.connection.clone();

        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await
            .context("Redis health check failed")?;

        Ok(())
    }

    /// Registers a delivery exactly once.
    ///
    /// Returns true when the delivery was newly registered.
    /// Returns false when the delivery already exists.
    pub async fn register_delivery(&self, request: &DeliveryRequest) -> anyhow::Result<bool> {
        let mut connection = self.connection.clone();

        let state_key = keys::delivery_state(
            request.tenant_id,
            request.destination_id,
            request.delivery_id,
        );

        let request_key = keys::delivery_request(
            request.tenant_id,
            request.destination_id,
            request.delivery_id,
        );

        let request_json = serde_json::to_string(request)?;

        let script = Script::new(REGISTER_DELIVERY);

        let result: i64 = script
            .key(state_key)
            .key(request_key)
            .arg(request.event_id.to_string())
            .arg(request.topic_id.to_string())
            .arg(Utc::now().timestamp_millis())
            .arg(request_json)
            .invoke_async(&mut connection)
            .await
            .context("failed to register delivery")?;

        match result {
            1 => Ok(true),
            0 => Ok(false),
            -1 => bail!("delivery request exists without corresponding delivery state"),
            other => bail!("unexpected register_delivery result: {other}"),
        }
    }

    /// Attempts to acquire a processing lease.
    ///
    /// `lease_duration` should be longer than the maximum HTTP request
    /// timeout, with enough margin for Redis completion.
    pub async fn acquire_delivery(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        delivery_id: Uuid,
        attempt_id: Uuid,
        lease_duration: Duration,
    ) -> anyhow::Result<AcquireResult> {
        let mut connection = self.connection.clone();

        let state_key = keys::delivery_state(tenant_id, destination_id, delivery_id);

        let retry_key = keys::destination_retry(tenant_id, destination_id);

        let now = Utc::now().timestamp_millis();

        let lease_until = now.saturating_add(
            i64::try_from(lease_duration.as_millis())
                .context("lease duration exceeds supported range")?,
        );

        let script = Script::new(ACQUIRE_DELIVERY);

        let result: i64 = script
            .key(state_key)
            .key(retry_key)
            .arg(now)
            .arg(lease_until)
            .arg(attempt_id.to_string())
            .arg(delivery_id.to_string())
            .invoke_async(&mut connection)
            .await
            .context("failed to acquire delivery")?;

        match result {
            attempt if attempt > 0 => {
                let attempt =
                    u32::try_from(attempt).context("delivery attempt count exceeds u32")?;

                Ok(AcquireResult::Acquired { attempt })
            }
            0 => Ok(AcquireResult::Busy),
            -1 => Ok(AcquireResult::Missing),
            -2 => Ok(AcquireResult::AlreadySucceeded),
            -3 => Ok(AcquireResult::AlreadyFailed),
            -4 => Ok(AcquireResult::InvalidState),
            other => bail!("unexpected acquire_delivery result: {other}"),
        }
    }

    /// Returns due retry delivery IDs.
    ///
    /// This method does not remove them from the retry set. The subsequent
    /// acquire_delivery call performs the exclusive processing acquisition.
    pub async fn claim_due_retries(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        limit: usize,
    ) -> anyhow::Result<Vec<Uuid>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut connection = self.connection.clone();

        let retry_key = keys::destination_retry(tenant_id, destination_id);

        let state_prefix = keys::destination_delivery_state_prefix(tenant_id, destination_id);

        let script = Script::new(CLAIM_RETRY);

        let ids: Vec<String> = script
            .key(retry_key)
            .arg(Utc::now().timestamp_millis())
            .arg(limit)
            .arg(state_prefix)
            .invoke_async(&mut connection)
            .await
            .context("failed to claim due retries")?;

        ids.into_iter()
            .map(|id| Uuid::parse_str(&id).context("retry set contains an invalid delivery ID"))
            .collect()
    }

    /// Atomically completes an attempt, updates delivery state,
    /// schedules a retry or records a pending DLQ message, and emits
    /// the outcome to DELIVERY_RESULTS.
    pub async fn complete_attempt(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        outcome: &DeliveryOutcome,
        next_attempt_at: Option<DateTime<Utc>>,
        request: &DeliveryRequest,
    ) -> anyhow::Result<CompleteAttemptResult> {
        let mut connection = self.connection.clone();

        let state_key = keys::delivery_state(tenant_id, destination_id, outcome.delivery_id);

        let retry_key = keys::destination_retry(tenant_id, destination_id);

        let dlq_pending_key = keys::destination_dlq_pending(tenant_id, destination_id);

        let outcome_json = serde_json::to_string(outcome)?;
        let registry_member = keys::pending_dlq_destination(tenant_id, destination_id);

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
            .key(keys::PENDING_DLQ_DESTINATIONS)
            .arg(outcome.attempt_id.to_string())
            .arg(outcome_json)
            .arg(if outcome.success { 1 } else { 0 })
            .arg(if outcome.will_retry { 1 } else { 0 })
            .arg(outcome.delivery_id.to_string())
            .arg(next_attempt_ms)
            .arg(dlq_json)
            .arg(registry_member)
            .invoke_async(&mut connection)
            .await
            .context("failed to complete delivery attempt")?;

        match result {
            1 => Ok(CompleteAttemptResult::Completed),
            -1 => Ok(CompleteAttemptResult::Missing),
            -2 => Ok(CompleteAttemptResult::NotProcessing),
            -3 => Ok(CompleteAttemptResult::StaleAttempt),
            other => bail!("unexpected complete_attempt result: {other}"),
        }
    }

    pub async fn list_pending_dlq_destinations(
        &self,
    ) -> anyhow::Result<Vec<PendingDlqDestination>> {
        let mut connection = self.connection.clone();

        let members: Vec<String> = connection
            .smembers(keys::PENDING_DLQ_DESTINATIONS)
            .await
            .context("failed to read pending DLQ destination registry")?;

        members
            .into_iter()
            .map(|member| {
                let (tenant_id, destination_id) = member
                    .split_once(':')
                    .context("invalid pending DLQ destination registry member")?;

                Ok(PendingDlqDestination {
                    tenant_id: Uuid::parse_str(tenant_id)
                        .context("invalid tenant ID in DLQ registry")?,
                    destination_id: Uuid::parse_str(destination_id)
                        .context("invalid destination ID in DLQ registry")?,
                })
            })
            .collect()
    }
    pub async fn get_delivery_request(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        delivery_id: Uuid,
    ) -> anyhow::Result<Option<DeliveryRequest>> {
        let mut connection = self.connection.clone();

        let raw: Option<String> = connection
            .get(keys::delivery_request(
                tenant_id,
                destination_id,
                delivery_id,
            ))
            .await
            .context("failed to get delivery request")?;

        raw.map(|value| {
            serde_json::from_str(&value).context("stored delivery request is invalid JSON")
        })
        .transpose()
    }
    /// Lists destinations that have pending DLQ messages.
    pub async fn get_pending_dlq_messages(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        limit: usize,
    ) -> anyhow::Result<Vec<(Uuid, DlqMessage)>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut connection = self.connection.clone();
        let key = keys::destination_dlq_pending(tenant_id, destination_id);

        let mut cursor = 0_u64;
        let mut messages = Vec::with_capacity(limit);

        loop {
            let (next_cursor, entries): (u64, Vec<(String, String)>) = redis::cmd("HSCAN")
                .arg(&key)
                .arg(cursor)
                .arg("COUNT")
                .arg(limit - messages.len())
                .query_async(&mut connection)
                .await
                .context("failed to scan pending DLQ messages")?;

            for (raw_id, raw_message) in entries {
                let delivery_id = Uuid::parse_str(&raw_id).with_context(|| {
                    format!("invalid delivery ID in DLQ pending hash: {raw_id}")
                })?;

                let message: DlqMessage =
                    serde_json::from_str(&raw_message).context("invalid serialized DLQ message")?;

                messages.push((delivery_id, message));

                if messages.len() >= limit {
                    break;
                }
            }

            if messages.len() >= limit || next_cursor == 0 {
                break;
            }

            cursor = next_cursor;
        }

        Ok(messages)
    }
    /// Removes a pending DLQ marker after successful broker publication.
    ///
    /// The hash entry and global registry membership are updated atomically.
    pub async fn acknowledge_dlq_message(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        delivery_id: Uuid,
    ) -> anyhow::Result<()> {
        let mut connection = self.connection.clone();

        let pending_key = keys::destination_dlq_pending(tenant_id, destination_id);

        let registry_member = keys::pending_dlq_destination(tenant_id, destination_id);

        let script = Script::new(ACK_DLQ_MESSAGE);

        let _: i64 = script
            .key(pending_key)
            .key(keys::PENDING_DLQ_DESTINATIONS)
            .arg(delivery_id.to_string())
            .arg(registry_member)
            .invoke_async(&mut connection)
            .await
            .context("failed to acknowledge DLQ message")?;

        Ok(())
    }
}
