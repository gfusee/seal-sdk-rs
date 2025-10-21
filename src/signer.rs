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

use crate::generic_types::SuiAddress;
use async_trait::async_trait;
use fastcrypto::ed25519::{Ed25519PublicKey, Ed25519Signature};

/// Abstraction over the minimal signing capabilities needed to mint `SessionKey`s.
///
/// The trait captures the ability to produce personal-message signatures together with
/// the caller's public key and Sui address. When the crate is compiled with the relevant
/// feature flags, an implementation for `sui_sdk::wallet_context::WalletContext` is
/// provided out of the box.
#[async_trait]
pub trait Signer {
    type Error;

    async fn sign_personal_message(
        &mut self,
        message: Vec<u8>,
    ) -> Result<Ed25519Signature, Self::Error>;

    fn get_public_key(&mut self) -> Result<Ed25519PublicKey, Self::Error>;

    fn get_sui_address(&mut self) -> Result<SuiAddress, Self::Error> {
        Ok(SuiAddress([0; 32]))
    }
}
