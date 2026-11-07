use std::{sync::Arc, time::Duration};

use thiserror::Error;

#[derive(Clone)]
pub struct PublicKeyStore {
    pem: Arc<str>,
}

impl PublicKeyStore {
    pub fn from_pem(pem: impl Into<Arc<str>>) -> Self {
        Self { pem: pem.into() }
    }

    pub async fn fetch(url: &str) -> Result<Self, PublicKeyError> {
        let response = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let body: PublicKeyResponse = response.text().await?;

        if body.trim().is_empty() {
            return Err(PublicKeyError::EmptyKey);
        }

        Ok(Self::from_pem(body))
    }

    pub fn pem(&self) -> &str {
        &self.pem
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.pem.as_bytes()
    }
}

type PublicKeyResponse = String;

#[derive(Debug, Error)]
pub enum PublicKeyError {
    #[error("failed to fetch IAM public key: {0}")]
    Request(#[from] reqwest::Error),

    #[error("IAM returned an empty public key")]
    EmptyKey,
}
