use async_trait::async_trait;
use serde_json::Value;
use sui_sdk::rpc_types::{SuiMoveValue, SuiParsedData};
use sui_types::dynamic_field::DynamicFieldName;
use sui_types::TypeTag;
use thiserror::Error;
use crate::base_client::KeyServerInfo;
use crate::generic_types::ObjectID;
use crate::sui_client::SuiClient;

#[derive(Debug, Error)]
pub enum SuiClientError {
    #[error("Sui SDK error: {0}")]
    SuiSdk(#[from] sui_sdk::error::Error),

    #[error("No object data from the Sui RPC for object {object_id}")]
    NoObjectDataFromTheSuiRPC { object_id: sui_types::base_types::ObjectID },

    #[error("Invalid object data from the Sui RPC for object {object_id}")]
    InvalidObjectDataFromTheSuiRPC { object_id: sui_types::base_types::ObjectID },

    #[error("Invalid dynamic fields type from key server for object {object_id}")]
    InvalidKeyServerDynamicFieldsType { object_id: sui_types::base_types::ObjectID },

    #[error("Missing key server field: {field_name}")]
    MissingKeyServerField { field_name: String },
}

#[async_trait]
impl SuiClient for sui_sdk::SuiClient {
    type Error = SuiClientError;

    async fn get_key_server_info(
        &self,
        key_server_id: [u8; 32],
    ) -> Result<KeyServerInfo, Self::Error> {
        let key_server_id = sui_types::base_types::ObjectID::new(key_server_id);

        let dynamic_field_name = DynamicFieldName {
            type_: TypeTag::U64,
            value: Value::String("1".to_string()),
        };

        let response = self
            .read_api()
            .get_dynamic_field_object(
                key_server_id,
                dynamic_field_name
            )
            .await?;

        let object_data = response.data.ok_or_else(|| {
            SuiClientError::NoObjectDataFromTheSuiRPC {
                object_id: key_server_id,
            }
        })?;

        let content = object_data.content.ok_or_else(|| {
            SuiClientError::NoObjectDataFromTheSuiRPC {
                object_id: key_server_id,
            }
        })?;

        let parsed = match content {
            SuiParsedData::MoveObject(obj) => obj,
            _ => {
                return Err(SuiClientError::InvalidObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                })
            }
        };

        let error_no_move_field = |field_name: &str| {
            SuiClientError::MissingKeyServerField { field_name: field_name.to_string() }
        };

        let value_field = parsed.fields
            .field_value("value")
            .ok_or_else(|| error_no_move_field("value"))?;

        let value_struct = match value_field {
            SuiMoveValue::Struct(value_struct) => value_struct,
            _ => return Err(SuiClientError::InvalidKeyServerDynamicFieldsType { object_id: key_server_id }),
        };

        let url_value = value_struct
            .field_value("url")
            .ok_or_else(|| error_no_move_field("url"))?;

        let name_value = value_struct
            .field_value("name")
            .ok_or_else(|| error_no_move_field("name"))?;

        let public_key_value = value_struct
            .field_value("pk")
            .ok_or_else(|| error_no_move_field("pk"))?;

        let (url, name, public_key) = match (url_value, name_value, public_key_value) {
            (SuiMoveValue::String(url), SuiMoveValue::String(name), SuiMoveValue::Vector(public_key_values)) => {
                let public_key_bytes = public_key_values
                    .into_iter()
                    .map(|value| {
                        match value {
                            SuiMoveValue::Number(byte) => {
                                match u8::try_from(byte) {
                                    Ok(byte) => Ok(byte),
                                    Err(_) => Err(SuiClientError::InvalidKeyServerDynamicFieldsType { object_id: key_server_id }),
                                }
                            },
                            _ => Err(SuiClientError::InvalidKeyServerDynamicFieldsType { object_id: key_server_id }),
                        }
                    })
                    .collect::<Result<Vec<u8>, _>>()?;

                let public_key = hex::encode(&public_key_bytes);

                (url, name, public_key)
            }
            _ => return Err(SuiClientError::InvalidKeyServerDynamicFieldsType { object_id: key_server_id }),
        };

        let key_server_info = KeyServerInfo {
            object_id: ObjectID(key_server_id.into_bytes()),
            name,
            url,
            public_key,
        };

        Ok(key_server_info)
    }
}