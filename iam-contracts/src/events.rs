use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Current schema version for tenant projection events.
pub const TENANT_PROJECTION_SCHEMA_VERSION: i16 = 1;

/// Event type stored in the PostgreSQL outbox.
pub const TENANT_PROJECTION_EVENT: &str = "tenant.projection.refresh";

/// A request to rebuild the Redis projection for a tenant.
///
/// This event deliberately carries no projection data. The worker reads the
/// latest authoritative state from PostgreSQL when processing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantProjectionEvent {
    pub tenant_id: Uuid,
}

impl TenantProjectionEvent {
    pub fn new(tenant_id: Uuid) -> Self {
        Self { tenant_id }
    }
}
