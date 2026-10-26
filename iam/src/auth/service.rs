use std::sync::Arc;

use iam_contracts::auth::jwt::signer::JwtSigner;

use crate::{
    auth::{
        dto::{LoginRequest, LoginResponse},
        model::Claims,
    },
    config::Config,
    domain::{membership::repository::MembershipRepository, user::repository::UserRepository},
    shared::{
        auth::crypto::verify_password,
        error::{ApiError, ApiResult},
    },
};

pub async fn login(
    user_repo: &Arc<dyn UserRepository>,
    membership_repo: &Arc<dyn MembershipRepository>,
    config: &Config,
    jwt_signer: Arc<JwtSigner>,
    req: LoginRequest,
) -> ApiResult<LoginResponse> {
    let err = || ApiError::BadRequest("Invalid username or password".into());
    let user = user_repo
        .get_by_username(&req.username)
        .await?
        .ok_or(err())?;
    let pass_status = verify_password(&req.password, &user.password_hash)?;
    if !pass_status {
        return Err(err());
    }
    let user_memberships = membership_repo.list(user.id).await?;
    let claims = Claims::new(&user, user_memberships.memberships, &config.auth_config);
    let token = jwt_signer.sign(&claims)?;
    Ok(LoginResponse {
        username: req.username,
        email: user.email,
        token: token,
    })
}
