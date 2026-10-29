use uuid::Uuid;

/// Logical event stream for a tenant and topic.
///
/// This is a backend-agnostic destination identifier.
/// The queue implementation decides how it is physically stored.
pub fn topic_events(tenant_id: Uuid, topic_id: Uuid) -> String {
    return format!("v1:tenant:{tenant_id}:topic:{topic_id}:events");
}

/// Stream for delivery results produced by outbound workers.
pub const DELIVERY_RESULTS: &str = "v1:delivery-results";
