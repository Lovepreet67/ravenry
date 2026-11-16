use metrics::describe_counter;

// Counters
pub const EVENTS_RECEIVED: &str = "ravenary_events_received_total";
pub const EVENTS_PUBLISHED: &str = "ravenary_events_published_total";
pub const MANUAL_RETRY_RECEIVED: &str = "ravenary_manual_retry_received_total";
pub const MANUAL_RETRY_PUBLISHED: &str = "ravenary_manual_retry_published_total";
pub const DELIVERIES_CREATED: &str = "ravenary_deliveries_created_total";
pub const DELIVERIES_ATTEMPTS: &str = "ravenary_deliveries_attempts_total";

pub const HTTP_REQUESTS: &str = "ravenary_http_requests_total";
pub const HTTP_DURATION: &str = "ravenary_http_request_duration_seconds";

pub fn describe_all() {
    describe_counter!(EVENTS_RECEIVED, "Total events recieved");
    describe_counter!(EVENTS_PUBLISHED, "Total events published");
    describe_counter!(DELIVERIES_CREATED, "Total deliveries created");
    describe_counter!(DELIVERIES_ATTEMPTS, "Total delivery recieved");
}
