use crate::generic_types::SuiAddress;
use async_trait::async_trait;
use fastcrypto::ed25519::{Ed25519PublicKey, Ed25519Signature};

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
