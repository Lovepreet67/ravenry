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
