// Copyright 2025 Quentin Diebold
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
