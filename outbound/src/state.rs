use std::{collections::HashSet, sync::Arc};

use iam_contracts::auth::jwt::verifier::JwtVerifier;
use streamforge::traits::Queue;

use crate::{config::Config, platform::projection_store::ProjectionStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub queue: Arc<dyn Queue>,
    pub publish_roles: Arc<HashSet<String>>,
    pub projections: ProjectionStore,
    pub jwt_verifier: Arc<JwtVerifier>,
}
