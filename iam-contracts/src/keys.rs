use uuid::Uuid;

/// Current Redis projection namespace version.
pub const KEY_PREFIX: &str = "v1";

/// Global projection readiness marker.
pub const PROJECTION_VERSION_KEY: &str = "v1:projection:version";

/// Redis key for tenant metadata.
///
/// Hash fields:
/// - `status`
/// - `slug`
/// - `version`
pub fn tenant_meta(tenant_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:meta");
}

/// Redis key mapping topic slugs to topic UUIDs.
///
/// Hash fields are topic slugs; values are topic UUIDs.
pub fn tenant_topics(tenant_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:topics");
}

/// Redis set containing destination UUIDs subscribed to a topic.
pub fn topic_subscriptions(tenant_id: Uuid, topic_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:sub:{topic_id}");
}

/// Redis hash containing a destination's projected configuration.
///
/// Hash fields may include:
/// - `url`
/// - `secret`
/// - `kind`
/// - `status`
/// - `slug`
/// - `version`
pub fn destination(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}");
}

/// Redis set containing all keys owned by a tenant's projection.
///
/// Used to atomically replace or remove a tenant's projection without
/// scanning the entire Redis database.
pub fn tenant_projection_keys(tenant_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:projection-keys");
}

/// Registry of active tenant/topic pairs available for event consumption.
pub const ACTIVE_TOPIC_STREAMS: &str = "v1:registry:active-topic-streams";

/// Redis hash containing the current state of a delivery.
///
/// Fields:
/// - `status`
/// - `attempts`
/// - `attempt_id`
/// - `lease_until`
/// - `last_error`
/// - `next_attempt_at`
pub fn delivery_state(tenant_id: Uuid, destination_id: Uuid, delivery_id: Uuid) -> String {
    return format!(
        "{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:delivery:{delivery_id}:state"
    );
}

/// Redis string containing the serialized request needed for retries.
pub fn delivery_request(tenant_id: Uuid, destination_id: Uuid, delivery_id: Uuid) -> String {
    return format!(
        "{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:delivery:{delivery_id}:request"
    );
}

/// Sorted set of scheduled retries for a destination.
///
/// Score: next attempt timestamp in milliseconds.
/// Member: delivery_id.
pub fn destination_retry(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:retry");
}

/// Stream containing terminally failed deliveries for a destination.
pub fn destination_dlq(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:dlq");
}

/// Redis hash containing DLQ messages that still need to be
/// published to the StreamForge DLQ queue.
///
/// Field: delivery_id
/// Value: serialized DlqMessage
pub fn destination_dlq_pending(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:dlq-pending");
}

/// Prefix used by Lua scripts to construct per-delivery state keys.
pub fn destination_delivery_state_prefix(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{KEY_PREFIX}:tenant:{tenant_id}:dest:{destination_id}:delivery:");
}

/// Set containing destinations that have pending DLQ messages.
///
/// Members use the format: tenant_id:destination_id
pub const PENDING_DLQ_DESTINATIONS: &str = "v1:registry:pending-dlq-destinations";

pub fn pending_dlq_destination(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("{tenant_id}:{destination_id}");
}
