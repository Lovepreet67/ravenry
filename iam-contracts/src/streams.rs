use uuid::Uuid;

/// Logical event stream for a tenant and topic.
///
/// This is a backend-agnostic destination identifier.
/// The queue implementation decides how it is physically stored.
pub fn topic_events(tenant_id: Uuid, topic_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:topic:{topic_id}:events");
}

/// Logical event stream for a tenant and topic DLQ.
pub fn topic_events_dlq(tenant_id: Uuid, topic_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:topic:{topic_id}:dlq");
}

/// Stream for delivery results produced by outbound workers.
pub const DELIVERY_RESULTS: &str = "v1:delivery-results";

pub const MANUALY_RETRY: &str = "v1:manual-retry";

/// DLQ stream for a specific tenant and destination.
pub fn destination_dlq(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:dest:{destination_id}:dlq");
}
/// DLQ stream for a specific tenant and destination.
pub fn destination_outcomes(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:dest:{destination_id}:outcome");
}

pub fn destination_deliveries(tenant_id: Uuid, destination_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:dest:{destination_id}:deliveries");
}
