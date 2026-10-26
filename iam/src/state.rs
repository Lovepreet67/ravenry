use std::sync::Arc;

use iam_contracts::auth::jwt::{signer::JwtSigner, verifier::JwtVerifier};

use crate::{
    config::Config,
    domain::{
        destination::repository::DestinationRepository,
        membership::repository::MembershipRepository, role::repository::RoleRepository,
        tenant::repository::TenantRepository, topic::repository::TopicRepository,
        user::repository::UserRepository,
    },
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub membership_repo: Arc<dyn MembershipRepository>,
    pub role_repo: Arc<dyn RoleRepository>,
    pub destination_repo: Arc<dyn DestinationRepository>,
    pub topic_repo: Arc<dyn TopicRepository>,
    pub config: Arc<Config>,
    pub jwt_verifier: Arc<JwtVerifier>,
    pub jwt_signer: Arc<JwtSigner>,
}
