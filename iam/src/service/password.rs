use std::sync::OnceLock;

use argon2::{
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::phc::SaltString,
};

use crate::error::ApiResult;

fn get_argon_hasher() -> &'static Argon2<'static> {
    static ARGON_INSTANCE: OnceLock<Argon2> = OnceLock::new();
    ARGON_INSTANCE.get_or_init(|| {
        // Safe, production-ready parameters (19 MiB memory, 2 iterations)
        let params = Params::new(19456, 2, 1, None).expect("Valid Argon2 parameters configuration");

        Argon2::new(
            argon2::Algorithm::Argon2id, // Standard variant for password hashing
            argon2::Version::V0x13,
            params,
        )
    })
}
pub fn create_password_hash(password: &str) -> String {
    let hasher = get_argon_hasher();
    let salt = SaltString::generate();
    match hasher.hash_password_with_salt(password.as_bytes(), salt.as_bytes()) {
        Ok(ph) => ph.to_string(),
        Err(e) => panic!("{:?}", e),
    }
}

pub fn verify_password(password: &str, hash: &str) -> ApiResult<bool> {
    let password_hash = PasswordHash::new(hash)?;
    let hasher = get_argon_hasher();
    Ok(hasher
        .verify_password(password.as_bytes(), &password_hash)
        .is_ok())
}
