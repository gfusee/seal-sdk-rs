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

impl BCSSerializableProgrammableTransaction
    for sui_sdk::types::transaction::ProgrammableTransaction
{
    fn to_bcs_bytes(&self) -> Result<Vec<u8>, SealClientError> {
        Ok(bcs::to_bytes(self)?)
    }
}
