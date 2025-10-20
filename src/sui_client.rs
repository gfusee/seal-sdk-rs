use crate::base_client::KeyServerInfo;
use async_trait::async_trait;
use std::fmt::Display;

#[async_trait]
pub trait SuiClient: Send + Sync {
    type Error: Display + Send + Sync;

    async fn get_key_server_info(
        &self,
        key_server_id: [u8; 32],
    ) -> Result<KeyServerInfo, Self::Error>;
}
