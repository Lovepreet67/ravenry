use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::TenantScope;
use crate::domain::delivery::{Delivery, DeliveryFilter};
use crate::domain::delivery_attempt::DeliveryAttempt;
use crate::domain::event::{EventDetail, EventFilter, EventSummary};
use crate::domain::pagination::{Page, PageRequest};
use crate::error::RepoError;

pub type RepoResult<T> = Result<T, RepoError>;

pub mod attempts;
pub mod deliveries;
pub mod events;
// pub mod stats;   // later

pub use attempts::PgAttemptRepository;
pub use deliveries::PgDeliveryRepository;
pub use events::PgEventRepository;

/// "Can this instance reach its storage?" Keeps the health endpoint free
/// of any database type.
#[async_trait]
pub trait Readiness: Send + Sync {
    async fn check(&self) -> Result<(), String>;
}

/// Every method takes a `&TenantScope`, and every implementation MUST
/// restrict results to `scope.tenant_id()`. A row owned by another tenant
/// is indistinguishable from a missing one (`None` / empty).
#[async_trait]
pub trait EventRepository: Send + Sync {
    async fn list(
        &self,
        scope: &TenantScope,
        filter: &EventFilter,
        page: &PageRequest,
    ) -> RepoResult<Page<EventSummary>>;

    async fn get(&self, scope: &TenantScope, event_id: Uuid) -> RepoResult<Option<EventDetail>>;
}

#[async_trait]
pub trait DeliveryRepository: Send + Sync {
    async fn list(
        &self,
        scope: &TenantScope,
        filter: &DeliveryFilter,
        page: &PageRequest,
    ) -> RepoResult<Page<Delivery>>;

    async fn get(&self, scope: &TenantScope, delivery_id: Uuid) -> RepoResult<Option<Delivery>>;
}

#[async_trait]
pub trait AttemptRepository: Send + Sync {
    /// Newest attempt first. Empty if the delivery is unknown OR belongs to
    /// another tenant, so handlers should call `DeliveryRepository::get`
    /// first and return 404 on `None`.
    async fn list_for_delivery(
        &self,
        scope: &TenantScope,
        delivery_id: Uuid,
    ) -> RepoResult<Vec<DeliveryAttempt>>;
}
