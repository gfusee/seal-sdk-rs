use crate::error::SessionKeyError;
use crate::generic_types::{ObjectID, SuiAddress};
use crate::signer::Signer;
use base64::Engine;
use chrono::{DateTime, Utc};
use seal_crypto::elgamal::genkey;
use fastcrypto::ed25519::{Ed25519KeyPair, Ed25519PublicKey};
use fastcrypto::traits::KeyPair;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use sui_sdk_types::{SimpleSignature, UserSignature};
use crate::crypto::{Certificate, ElGamalPublicKey, ElGamalSecretKey, ElgamalVerificationKey, FetchKeyRequest};

const MIN_TTL_MIN: u16 = 1;
const MAX_TTL_MAX: u16 = 30;

#[derive(Serialize, Deserialize)]
struct RequestFormat {
    ptb: Vec<u8>,
    enc_key: Vec<u8>,
    enc_verification_key: Vec<u8>,
}

pub struct SessionKey {
    address: SuiAddress,
    package_id: ObjectID,
    creation_time_ms: u64,
    ttl_min: u16,
    session_key: Ed25519KeyPair,
    personal_message_signer_address_and_public_key: (SuiAddress, Ed25519PublicKey),
    personal_message_signature: [u8; 64],
}

impl SessionKey {
    pub async fn new<ID, SigError, Sig>(
        package_id: ID,
        ttl_min: u16,
        signer: &mut Sig,
    ) -> Result<SessionKey, SessionKeyError>
    where
        ObjectID: From<ID>,
        SessionKeyError: From<SigError>,
        Sig: Signer<Error = SigError>
    {
        let package_id: ObjectID = package_id.into();

        if ttl_min < MIN_TTL_MIN || ttl_min > MAX_TTL_MAX {
            return Err(
                SessionKeyError::InvalidTTLMin {
                    min: MIN_TTL_MIN,
                    max: MAX_TTL_MAX,
                    received: ttl_min,
                }
            )
        };

        let signer_address = signer.get_sui_address()?;
        let signer_public_key = signer.get_public_key()?;

        let session_key = Ed25519KeyPair::generate(&mut thread_rng());

        let now_ms = Utc::now().timestamp_millis() as u64;

        let Some(message_to_sign) = signed_message(
            sui_sdk_types::ObjectId::from(package_id).to_string(),
            &session_key.public(),
            now_ms,
            ttl_min,
        ) else {
            return Err(SessionKeyError::CannotGenerateSignedMessage {
                package_id,
                creation_timestamp_ms: now_ms,
                ttl_min
            })
        };

        let signature = signer.sign_personal_message(
            message_to_sign.as_bytes().to_vec()
        ).await?;

        Ok(
            SessionKey {
                address: signer_address,
                package_id,
                creation_time_ms: chrono::Utc::now().timestamp_millis() as u64,
                ttl_min,
                session_key,
                personal_message_signer_address_and_public_key: (signer_address, signer_public_key),
                personal_message_signature: signature.sig.to_bytes(),
            }
        )
    }

    pub fn address(&self) -> &SuiAddress {
        &self.address
    }

    pub fn package_id(&self) -> &ObjectID {
        &self.package_id
    }

    pub fn get_fetch_key_request(
        &self,
        approve_transaction_data: Vec<u8>,
    ) -> Result<(FetchKeyRequest, ElGamalSecretKey), SessionKeyError> {
        let approve_transaction_data_base64 = base64::engine::general_purpose::STANDARD.encode(&approve_transaction_data);

        let (signed_request, enc_secret, enc_key, enc_verification_key) = self.get_signed_request(
            approve_transaction_data
        )?;

        let request_signature = fastcrypto::traits::Signer::sign(&self.session_key, &signed_request);

        let result = FetchKeyRequest {
            ptb: approve_transaction_data_base64,
            enc_key,
            enc_verification_key,
            request_signature,
            certificate: self.get_certificate(),
        };

        Ok((result, enc_secret))
    }

    fn get_signed_request(
        &self,
        approve_transaction_data: Vec<u8>,
    ) -> Result<(Vec<u8>, ElGamalSecretKey, ElGamalPublicKey, ElgamalVerificationKey), SessionKeyError> {
        let keys: (_, ElGamalPublicKey, ElgamalVerificationKey) = genkey(&mut rand::thread_rng());

        let req = RequestFormat {
            ptb: approve_transaction_data,
            enc_key: bcs::to_bytes(&keys.1)?,
            enc_verification_key: bcs::to_bytes(&keys.2)?
        };

        Ok((bcs::to_bytes(&req)?, keys.0, keys.1, keys.2))
    }

    fn get_certificate(
        &self
    ) -> Certificate {
        let personal_message_signature = self.personal_message_signature;

        Certificate {
            user: self.personal_message_signer_address_and_public_key.0.into(),
            session_vk: self.session_key.public().clone(),
            creation_time: self.creation_time_ms,
            ttl_min: self.ttl_min,
            signature: UserSignature::Simple(SimpleSignature::Ed25519 {
                signature: sui_sdk_types::Ed25519Signature::from_bytes(
                    &personal_message_signature,
                )
                    .unwrap(),
                public_key: sui_sdk_types::Ed25519PublicKey::new(
                    self.personal_message_signer_address_and_public_key.1.0.to_bytes()
                ),
            }),
            mvr_name: None,
        }
    }
}

pub fn signed_message(
    package_name: String,
    vk: &Ed25519PublicKey,
    creation_time: u64,
    ttl_min: u16,
) -> Option<String> {
    let res = format!(
        "Accessing keys of package {} for {} mins from {}, session key {}",
        package_name,
        ttl_min,
        DateTime::<Utc>::from_timestamp((creation_time / 1000) as i64, 0)?,
        vk,
    );

    Some(res)
}
