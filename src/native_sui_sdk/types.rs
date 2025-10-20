use crate::error::SealClientError;
use crate::generic_types::{BCSSerializableProgrammableTransaction, ObjectID, SuiAddress};

impl From<SuiAddress> for sui_sdk::types::base_types::SuiAddress {
    fn from(value: SuiAddress) -> Self {
        Self::from(sui_sdk::types::base_types::ObjectID::new(value.0))
    }
}

impl From<sui_sdk::types::base_types::SuiAddress> for SuiAddress {
    fn from(value: sui_sdk::types::base_types::SuiAddress) -> SuiAddress {
        SuiAddress(value.to_inner())
    }
}

impl From<ObjectID> for sui_sdk::types::base_types::ObjectID {
    fn from(value: ObjectID) -> Self {
        Self::new(value.0)
    }
}

impl From<sui_sdk::types::base_types::ObjectID> for ObjectID {
    fn from(value: sui_sdk::types::base_types::ObjectID) -> ObjectID {
        ObjectID(value.into_bytes())
    }
}

impl BCSSerializableProgrammableTransaction for sui_sdk::types::transaction::ProgrammableTransaction {
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, SealClientError> {
        Ok(bcs::to_bytes(self)?)
    }
}