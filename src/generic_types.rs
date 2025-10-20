use crate::error::SealClientError;
use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub struct ObjectID(pub [u8; 32]);

impl From<[u8; 32]> for ObjectID {
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

impl FromStr for ObjectID {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        sui_sdk_types::ObjectId::from_str(s)
            .map(Into::into)
            .map_err(|_| anyhow!("Failed to parse ObjectID: {s}"))
    }
}

impl Display for ObjectID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        sui_sdk_types::ObjectId::from(*self).fmt(f)
    }
}

impl Serialize for ObjectID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        sui_sdk_types::ObjectId::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ObjectID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        sui_sdk_types::ObjectId::deserialize(deserializer).map(Self::from)
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
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

impl Serialize for SuiAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        sui_sdk_types::Address::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SuiAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        sui_sdk_types::Address::deserialize(deserializer).map(Self::from)
    }
}

pub trait BCSSerializableProgrammableTransaction {
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, SealClientError>;
}
