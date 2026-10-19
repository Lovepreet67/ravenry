use std::sync::Arc;

use crate::{
    config::Config,
    domain::{
        membership::repository::MembershipRepository, tenant::repository::TenantRepository,
        user::repository::UserRepository,
    },
};

#[derive(Clone, Debug)]
pub struct AppState {
    pub tenant_repo: Arc<dyn TenantRepository>,
    pub user_repo: Arc<dyn UserRepository>,
    pub membership_repo: Arc<dyn MembershipRepository>,
    pub config: Arc<Config>,
}
