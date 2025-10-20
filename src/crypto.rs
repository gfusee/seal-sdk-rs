use crate::generic_types::{ObjectID, SuiAddress};
use fastcrypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use fastcrypto::encoding::{Encoding, Hex};
use fastcrypto::error::{FastCryptoError, FastCryptoResult};
use fastcrypto::groups::GroupElement;
use fastcrypto::groups::bls12381::G2Element;
use seal_crypto::elgamal::{PublicKey, SecretKey, VerificationKey};
use seal_crypto::ibe::{UserSecretKey, verify_user_secret_key};
use seal_crypto::{
    Ciphertext, IBEEncryptions, IBEPublicKeys, IBEUserSecretKeys, create_full_id, elgamal, ibe,
    seal_decrypt,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use sui_sdk_types::UserSignature;

pub type ElGamalPublicKey = PublicKey<UserSecretKey>;
pub type ElgamalEncryption = Encryption<UserSecretKey>;
pub type ElgamalVerificationKey = VerificationKey<ibe::PublicKey>;
pub type ElGamalSecretKey = SecretKey<fastcrypto::groups::bls12381::G1Element>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EncryptedObject {
    pub version: u8,
    pub package_id: ObjectID,
    pub id: Vec<u8>,
    pub services: Vec<(ObjectID, u8)>,
    pub threshold: u8,
    pub encrypted_shares: IBEEncryptions,
    pub ciphertext: Ciphertext,
}

impl From<seal_crypto::EncryptedObject> for EncryptedObject {
    fn from(value: seal_crypto::EncryptedObject) -> Self {
        let services = value
            .services
            .into_iter()
            .map(|e| (e.0.into(), e.1))
            .collect();

        Self {
            version: value.version,
            package_id: value.package_id.into(),
            id: value.id,
            services,
            threshold: value.threshold,
            encrypted_shares: value.encrypted_shares,
            ciphertext: value.ciphertext,
        }
    }
}

impl From<EncryptedObject> for seal_crypto::EncryptedObject {
    fn from(value: EncryptedObject) -> Self {
        let services = value
            .services
            .into_iter()
            .map(|e| (e.0.into(), e.1))
            .collect();

        Self {
            version: value.version,
            package_id: value.package_id.into(),
            id: value.id,
            services,
            threshold: value.threshold,
            encrypted_shares: value.encrypted_shares,
            ciphertext: value.ciphertext,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Certificate {
    pub user: SuiAddress,
    pub session_vk: Ed25519PublicKey,
    pub creation_time: u64,
    pub ttl_min: u16,
    pub signature: UserSignature,
    pub mvr_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FetchKeyRequest {
    pub ptb: String,
    pub enc_key: ElGamalPublicKey,
    pub enc_verification_key: ElgamalVerificationKey,
    pub request_signature: Ed25519Signature,
    pub certificate: Certificate,
}

impl FetchKeyRequest {
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        let sig_base64 = self.certificate.signature.to_base64();

        let json = serde_json::json!({
            "ptb": self.ptb,
            "enc_key": self.enc_key,
            "enc_verification_key": self.enc_verification_key,
            "request_signature": self.request_signature,
            "certificate": {
                "user": self.certificate.user,
                "session_vk": self.certificate.session_vk,
                "creation_time": self.certificate.creation_time,
                "ttl_min": self.certificate.ttl_min,
                "signature": sig_base64,
                "mvr_name": self.certificate.mvr_name,
            }
        });

        serde_json::to_string(&json)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Encryption<G: GroupElement>(pub G, pub G);

impl<G: GroupElement> From<Encryption<G>> for elgamal::Encryption<G> {
    fn from(value: Encryption<G>) -> Self {
        Self(value.0, value.1)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DecryptionKey {
    pub id: Vec<u8>,
    pub encrypted_key: ElgamalEncryption,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FetchKeyResponse {
    pub decryption_keys: Vec<DecryptionKey>,
}

// This function comes from the seal repo, it avoids recompiling the entire seal-sdk crate
pub fn seal_decrypt_all_objects(
    enc_secret: &SecretKey<fastcrypto::groups::bls12381::G1Element>,
    seal_responses: &[(ObjectID, FetchKeyResponse)],
    encrypted_objects: Vec<EncryptedObject>,
    server_pk_map: &HashMap<ObjectID, G2Element>,
) -> FastCryptoResult<Vec<Vec<u8>>> {
    if encrypted_objects.is_empty() {
        return Ok(Vec::new());
    }
    if seal_responses.is_empty() {
        return Err(FastCryptoError::GeneralError(
            "No seal responses provided".to_string(),
        ));
    }

    let mut cached_keys: HashMap<Vec<u8>, HashMap<ObjectID, UserSecretKey>> = HashMap::new();
    let mut processed_servers: HashSet<ObjectID> = HashSet::new();

    for (server_id, seal_response) in seal_responses.iter() {
        if !processed_servers.insert(*server_id) {
            return Err(FastCryptoError::GeneralError(format!(
                "Duplicate server_id {} in seal_responses",
                server_id
            )));
        }

        let public_key = server_pk_map.get(server_id).ok_or_else(|| {
            FastCryptoError::GeneralError(format!(
                "No public key configured for server {}",
                server_id
            ))
        })?;

        for decryption_key in seal_response.decryption_keys.iter() {
            let user_secret_key =
                elgamal::decrypt(enc_secret, &decryption_key.encrypted_key.into());
            verify_user_secret_key(&user_secret_key, &decryption_key.id, public_key)?;

            cached_keys
                .entry(decryption_key.id.clone())
                .or_default()
                .insert(*server_id, user_secret_key);
        }
    }

    let mut decrypted_results = Vec::with_capacity(encrypted_objects.len());
    for encrypted_object in encrypted_objects.into_iter() {
        let full_id = create_full_id(&encrypted_object.package_id.0, &encrypted_object.id);
        let keys_for_id = cached_keys.get(&full_id).ok_or_else(|| {
            FastCryptoError::GeneralError(format!(
                "No keys available for object with full_id {:?}",
                Hex::encode(&full_id)
            ))
        })?;

        let mut usks = HashMap::new();
        let mut pks = Vec::with_capacity(encrypted_object.services.len());
        for (server_id, _index) in encrypted_object.services.iter() {
            let user_secret_key = keys_for_id.get(&server_id).ok_or_else(|| {
                FastCryptoError::GeneralError(format!(
                    "Object requires key from server {} but no response was provided from that server",
                    server_id
                ))
            })?;
            usks.insert((*server_id).into(), *user_secret_key);

            let pk = server_pk_map.get(&server_id).ok_or_else(|| {
                FastCryptoError::GeneralError(format!(
                    "No public key configured for server {}",
                    server_id
                ))
            })?;
            pks.push(*pk);
        }

        if usks.len() < encrypted_object.threshold as usize {
            return Err(FastCryptoError::GeneralError(format!(
                "Insufficient keys for object: have {}, threshold requires {}",
                usks.len(),
                encrypted_object.threshold
            )));
        }

        let secret = seal_decrypt(
            &encrypted_object.into(),
            &IBEUserSecretKeys::BonehFranklinBLS12381(usks),
            Some(&IBEPublicKeys::BonehFranklinBLS12381(pks)),
        )?;

        decrypted_results.push(secret);
    }

    Ok(decrypted_results)
}
