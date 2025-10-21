use crate::base_client::KeyServerInfo;
use async_trait::async_trait;
use std::fmt::Display;

/// Abstraction over the Sui JSON-RPC calls needed by the seal client.
///
/// The trait mirrors the signatures exposed by `sui_sdk::SuiClient` and provides
/// just enough surface area for [`BaseSealClient`](crate::base_client::BaseSealClient) to
/// retrieve key-server metadata required during encryption and decryption workflows.
/// When the crate is built with the `client` and `native-sui-sdk` features (enabled
/// by default), an implementation backed by `sui_sdk::SuiClient` lives in
/// [`native_sui_sdk::client::sui_client`](crate::native_sui_sdk::client::sui_client).
#[async_trait]
pub trait SuiClient: Send + Sync {
    type Error: Display + Send + Sync;

    async fn get_key_server_info(
        &self,
        key_server_id: [u8; 32],
    ) -> Result<KeyServerInfo, Self::Error>;
}
