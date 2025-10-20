use std::fmt::Display;
use serde::{Deserialize, Serialize};
use crate::error::SealClientError;

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ObjectID(pub [u8; 32]);

impl From<[u8; 32]> for ObjectID
{
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl From<ObjectID> for sui_sdk_types::ObjectId {
    fn from(value: ObjectID) -> Self {
        Self::new(value.0)
    }
}

impl From<sui_sdk_types::ObjectId> for ObjectID {
    fn from(value: sui_sdk_types::ObjectId) -> Self {
        Self::from(value.into_inner())
    }
}

impl From<ObjectID> for seal_crypto::ObjectID {
    fn from(value: ObjectID) -> Self {
        Self::new(value.0)
    }
}

impl From<seal_crypto::ObjectID> for ObjectID {
    fn from(value: seal_crypto::ObjectID) -> Self {
        Self::from(value.into_inner())
    }
}

impl From<ObjectID> for sui_sdk_types::Address {
    fn from(value: ObjectID) -> Self {
        Self::new(value.0)
    }
}

impl From<sui_sdk_types::Address> for ObjectID {
    fn from(value: sui_sdk_types::Address) -> Self {
        Self::from(value.into_inner())
    }
}

impl Display for ObjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        sui_sdk_types::ObjectId::from(*self).fmt(f)
    }
}

#[cfg(feature = "native-sui-sdk")]
impl From<ObjectID> for sui_sdk::types::base_types::ObjectID {
    fn from(value: ObjectID) -> Self {
        Self::new(value.0)
    }
}

#[cfg(feature = "native-sui-sdk")]
impl From<sui_sdk::types::base_types::ObjectID> for ObjectID {
    fn from(value: sui_sdk::types::base_types::ObjectID) -> ObjectID {
        ObjectID(value.into_bytes())
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SuiAddress(pub [u8; 32]);

impl From<[u8; 32]> for SuiAddress {
    fn from(value: [u8; 32]) -> Self {
        Self(value.into())
    }
}

impl From<SuiAddress> for sui_sdk_types::Address {
    fn from(value: SuiAddress) -> Self {
        Self::new(value.0)
    }
}

impl From<sui_sdk_types::Address> for SuiAddress {
    fn from(value: sui_sdk_types::Address) -> Self {
        Self::from(value.into_inner())
    }
}

impl Display for SuiAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        sui_sdk_types::Address::from(*self).fmt(f)
    }
}

pub trait BCSSerializableProgrammableTransaction {
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, SealClientError>;
}