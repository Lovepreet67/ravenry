use std::sync::Arc;

use crate::{
    config::Config,
    domain::{
        membership::repository::MembershipRepository, role::repository::RoleRepository,
        tenant::repository::TenantRepository, user::repository::UserRepository,
    },
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub membership_repo: Arc<dyn MembershipRepository>,
    pub role_repo: Arc<dyn RoleRepository>,
    pub config: Arc<Config>,
}
