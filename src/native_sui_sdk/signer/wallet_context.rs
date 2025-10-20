use crate::generic_types::SuiAddress;
use crate::signer::Signer;
use async_trait::async_trait;
use fastcrypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use fastcrypto::traits::ToFromBytes;
use shared_crypto::intent::Intent;
use sui_keys::key_identity::KeyIdentity;
use sui_keys::keystore::{AccountKeystore, Keystore};
use sui_types::crypto::{Signature, SuiSignature};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletContextError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),

    #[error(transparent)]
    FastCryptoError(#[from] fastcrypto::error::FastCryptoError),

    #[error("Error while signing a message: {message}")]
    SignatureError { message: String },

    #[error("Incorrect signature scheme")]
    IncorrectSignatureScheme,
}

#[async_trait]
impl Signer for sui_sdk::wallet_context::WalletContext {
    type Error = WalletContextError;
    async fn sign_personal_message(
        &mut self,
        message: Vec<u8>,
    ) -> Result<Ed25519Signature, WalletContextError> {
        let generic_address = self.get_sui_address()?;
        let address = generic_address.into();
        let identity = KeyIdentity::Address(address);
        let keystore = self.get_keystore_by_identity(&identity)?;

        let signature = keystore
            .sign_secure(&address, &message, Intent::personal_message())
            .await
            .map_err(|err| WalletContextError::SignatureError {
                message: err.to_string(),
            })?;

        let Signature::Ed25519SuiSignature(signature) = signature else {
            return Err(WalletContextError::IncorrectSignatureScheme);
        };

        Ok(Ed25519Signature::from_bytes(signature.signature_bytes())?)
    }

    fn get_public_key(&mut self) -> Result<Ed25519PublicKey, WalletContextError> {
        let generic_address = self.get_sui_address()?;
        let address = generic_address.into();
        let identity = KeyIdentity::Address(address);
        let keystore = self.get_keystore_by_identity(&identity)?;
        let public_key = match keystore {
            Keystore::File(file_keystore) => file_keystore.export(&address)?.public(),
            Keystore::InMem(in_mem_keystore) => in_mem_keystore.export(&address)?.public(),
            Keystore::External(external_keystore) => external_keystore.export(&address)?.public(),
        };

        Ok(Ed25519PublicKey::from_bytes(public_key.as_ref())?)
    }

    fn get_sui_address(&mut self) -> Result<SuiAddress, WalletContextError> {
        Ok(SuiAddress(self.active_address()?.to_inner()))
    }
}
