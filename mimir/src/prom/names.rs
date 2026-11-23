use metrics::describe_counter;

// Counters
pub const EVENTS_RECEIVED: &str = "ravenary_events_received_total";
pub const EVENTS_PUBLISHED: &str = "ravenary_events_published_total";
pub const MANUAL_RETRY_RECEIVED: &str = "ravenary_manual_retry_received_total";
pub const MANUAL_RETRY_PUBLISHED: &str = "ravenary_manual_retry_published_total";
pub const DELIVERIES_CREATED: &str = "ravenary_deliveries_created_total";
pub const DELIVERIES_ATTEMPTS: &str = "ravenary_deliveries_attempts_total";

// REGARDING MUNINN
pub const QUEUE_DELIVERIES_RECEIVED: &str = "queue_deliveries_recieved_total";
pub const QUEUE_MALFORMED_DELIVERIES_RECEIVED: &str = "queue_malformed_deliveries_recieved_total";
pub const MUNINN_INBOX_MESSAGES: &str = "muninn_inbox_messages_total";
pub const MUNINN_INBOX_DATABASE_ERRORS: &str = "muninn_inbox_database_errors_total";
pub const MUNINN_MESSAGES_PROCESSED: &str = "muninn_messages_processed_total";
pub const MUNINN_MESSAGES_PROCESSING_DURATION: &str = "muninn_processing_duration_seconds";
pub const MUNINN_MESSAGES_OUTCOME: &str = "muninn_messages_outcome_total";
pub const MUNINN_DATABASE_ERRORS: &str = "muninn_database_errors_total";
pub const MUNINN_BATCH_SIZE: &str = "muninn_batch_size";
pub const MUNINN_BATCH_CLAIM_ERRORS: &str = "muninn_batch_claim_errors";
pub const MUNINN_LEASE_LOSSES: &str = "muninn_lease_losses";

//
pub const HTTP_REQUESTS: &str = "ravenary_http_requests_total";
pub const HTTP_DURATION: &str = "ravenary_http_request_duration_seconds";

pub const METRIC_ACTIVE_WORKERS: &str = "worker_supervisor_active_workers";
pub fn describe_all() {
    describe_counter!(EVENTS_RECEIVED, "Total events recieved");
    describe_counter!(EVENTS_PUBLISHED, "Total events published");
    describe_counter!(DELIVERIES_CREATED, "Total deliveries created");
    describe_counter!(DELIVERIES_ATTEMPTS, "Total delivery recieved");
}
