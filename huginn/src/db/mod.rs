pub mod pool;

use crate::error::RepoError;

/// The single place sqlx errors become storage-agnostic errors.
pub fn map_err(e: sqlx::Error) -> RepoError {
    RepoError::Backend(e.to_string())
}
