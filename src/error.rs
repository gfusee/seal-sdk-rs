use fastcrypto::error::FastCryptoError;
use http::header::{InvalidHeaderName, InvalidHeaderValue};
use thiserror::Error;
use crate::generic_types::ObjectID;

#[derive(Debug, Error)]
pub enum SealClientError {
    #[error("Cannot unwrap typed error: {error_message}")]
    CannotUnwrapTypedError { error_message: String },

    #[error("FastCrypto error: {0}")]
    FastCrypto(#[from] FastCryptoError),

    #[error("BCS error: {0}")]
    BCS(#[from] bcs::Error),

    #[error("JSON serialization error: {0}")]
    JSONSerialization(#[from] serde_json::Error),

    #[error("HEX deserialization error: {0}")]
    HEXDeserialization(#[from] hex::FromHexError),

    #[error("Session key error error: {0}")]
    SessionKey(#[from] SessionKeyError),

    #[cfg(all(feature = "client", feature = "native-sui-sdk"))]
    #[error("Sui client error: {0}")]
    SuiClient(#[from] crate::native_sui_sdk::client::sui_client::SuiClientError),

    #[cfg(feature = "reqwest")]
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] ReqwestError),

    #[error("Error while fetching derived keys from {url}: HTTP {status} - {response}")]
    ErrorWhileFetchingDerivedKeys {
        url: String,
        status: u16,
        response: String,
    },

    #[error("Insufficient keys: received {received}, but threshold is {threshold}")]
    InsufficientKeys { received: usize, threshold: u8 },

    #[error("Missing decrypted object")]
    MissingDecryptedObject,

    #[error("Invalid public key {public_key}: {reason}")]
    InvalidPublicKey { public_key: String, reason: String },

    #[error("Unknown error: {0}")]
    UnknownError(#[from] anyhow::Error),
}

#[cfg(feature = "reqwest")]
#[derive(Debug, Error)]
pub enum ReqwestError {
    #[error("A reqwest error occurred: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Unable to convert http headers: InvalidHeaderValue")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("Unable to convert http headers: InvalidHeaderName")]
    InvalidHeaderName(#[from] InvalidHeaderName),
}

#[derive(Debug, Error)]
pub enum SessionKeyError {
    #[error("ttl_min should be a value between {min} and {max}, received {received}")]
    InvalidTTLMin { min: u16, max: u16, received: u16 },
    
    #[error("Cannot generate the certificate message for package {package_id}, for a duration of {ttl_min} minutes from {creation_timestamp_ms} (unix time in milliseconds)")]
    CannotGenerateSignedMessage {
        package_id: ObjectID,
        creation_timestamp_ms: u64,
        ttl_min: u16
    },
    
    #[error("BCS error: {0}")]
    BCS(#[from] bcs::Error),

    #[error("FastCrypto error: {0}")]
    FastCrypto(#[from] FastCryptoError),

    #[cfg(feature = "native-sui-sdk")]
    #[error("Wallet context error: {0}")]
    WalletContext(#[from] crate::native_sui_sdk::signer::wallet_context::WalletContextError),
}