use std::sync::Arc;

use iam_contracts::auth::jwt::verifier::JwtVerifier; // ADJUST: real type and path
use iam_contracts::auth::{context::AuthContext, error::AuthError};
use uuid::Uuid;

use crate::domain::TenantScope;
use crate::repo::{AttemptRepository, DeliveryRepository, EventRepository, Readiness};

/// Built once at startup and cloned per request. Everything inside is an
/// Arc, so cloning is cheap. Repositories are trait objects, so handlers
/// never know which database backs them.
#[derive(Clone)]
pub struct AppState {
    // Used by the AuthContext extractor: `state.jwt_verifier.verify_token(..)`.
    pub jwt_verifier: Arc<JwtVerifier>,

    pub events: Arc<dyn EventRepository>,
    pub deliveries: Arc<dyn DeliveryRepository>,
    pub attempts: Arc<dyn AttemptRepository>,

    payload_roles: Arc<Vec<String>>,
    pub ready: Arc<dyn Readiness>,
    // Added when written: pub iam: Arc<dyn IamClient>,
    //                     pub outbound: Arc<dyn OutboundClient>,
}

impl AppState {
    pub fn new(
        jwt_verifier: Arc<JwtVerifier>,
        events: Arc<dyn EventRepository>,
        deliveries: Arc<dyn DeliveryRepository>,
        attempts: Arc<dyn AttemptRepository>,
        ready: Arc<dyn Readiness>,
        payload_roles: Vec<String>,
    ) -> Self {
        Self {
            jwt_verifier,
            events,
            deliveries,
            attempts,
            ready,
            payload_roles: Arc::new(payload_roles),
        }
    }

    /// The only way a handler gets a `TenantScope`. The tenant id comes
    /// from the route, and is authorized against the token's memberships.
    /// A caller with no membership in the tenant gets a 403.
    pub fn scope(&self, auth: &AuthContext, tenant_id: Uuid) -> Result<TenantScope, AuthError> {
        TenantScope::authorize(auth, tenant_id, &self.payload_roles)
    }
}
