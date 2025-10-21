use async_trait::async_trait;
use std::collections::HashMap;

pub struct PostResponse {
    pub status: u16,
    pub text: String,
}

impl PostResponse {
    pub fn is_success(&self) -> bool {
        let status = self.status;

        status >= 200 && status < 300
    }
}

/// Thin wrapper around the HTTP capabilities required by the seal client.
///
/// Only simple POST semantics are needed to talk to key servers. When the crate's
/// `client` feature is enabled (the default), we provide an adapter for `reqwest::Client`
/// in [`reqwest::client`](crate::reqwest::client).
#[async_trait]
pub trait HttpClient: Sync {
    type PostError;

    async fn post<S: ToString + Send + Sync>(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: S,
    ) -> Result<PostResponse, Self::PostError>;
}
