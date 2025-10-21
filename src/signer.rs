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
