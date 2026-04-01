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

use crate::base_client::{KeyServerInfo, PartialKeyServer, ServerType};
use crate::generic_types::ObjectID;
use crate::sui_client::SuiClient;
use async_trait::async_trait;
use serde_json::Value;
use sui_sdk::rpc_types::{SuiMoveValue, SuiParsedData};
use sui_types::TypeTag;
use sui_types::dynamic_field::DynamicFieldName;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SuiClientError {
    #[error("Sui SDK error: {0}")]
    SuiSdk(#[from] sui_sdk::error::Error),

    #[error("No object data from the Sui RPC for object {object_id}")]
    NoObjectDataFromTheSuiRPC {
        object_id: sui_types::base_types::ObjectID,
    },

    #[error("Invalid object data from the Sui RPC for object {object_id}")]
    InvalidObjectDataFromTheSuiRPC {
        object_id: sui_types::base_types::ObjectID,
    },

    #[error("Invalid dynamic fields type from key server for object {object_id}")]
    InvalidKeyServerDynamicFieldsType {
        object_id: sui_types::base_types::ObjectID,
    },

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

        // Try V2 first, fall back to V1 if V2 dynamic field doesn't exist.
        match self.get_key_server_info_v2(key_server_id).await {
            Ok(info) => {
                log::debug!(
                    "seal: resolved key server object_id={} type={:?}",
                    info.object_id,
                    info.server_type,
                );
                Ok(info)
            }
            Err(err) => {
                log::debug!(
                    "seal: V2 resolution failed for object_id={}, falling back to V1: {}",
                    key_server_id,
                    err,
                );
                let info = self.get_key_server_info_v1(key_server_id).await?;
                log::debug!(
                    "seal: resolved key server (V1) object_id={} type={:?}",
                    info.object_id,
                    info.server_type,
                );
                Ok(info)
            }
        }
    }
}

trait SuiClientKeyServerExt {
    async fn get_key_server_info_v1(
        &self,
        key_server_id: sui_types::base_types::ObjectID,
    ) -> Result<KeyServerInfo, SuiClientError>;

    async fn get_key_server_info_v2(
        &self,
        key_server_id: sui_types::base_types::ObjectID,
    ) -> Result<KeyServerInfo, SuiClientError>;
}

impl SuiClientKeyServerExt for sui_sdk::SuiClient {
    async fn get_key_server_info_v1(
        &self,
        key_server_id: sui_types::base_types::ObjectID,
    ) -> Result<KeyServerInfo, SuiClientError> {
        let dynamic_field_name = DynamicFieldName {
            type_: TypeTag::U64,
            value: Value::String("1".to_string()),
        };

        let response = self
            .read_api()
            .get_dynamic_field_object(key_server_id, dynamic_field_name)
            .await?;

        let object_data =
            response
                .data
                .ok_or_else(|| SuiClientError::NoObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                })?;

        let content =
            object_data
                .content
                .ok_or_else(|| SuiClientError::NoObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                })?;

        let parsed = match content {
            SuiParsedData::MoveObject(obj) => obj,
            _ => {
                return Err(SuiClientError::InvalidObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                });
            }
        };

        let error_no_move_field = |field_name: &str| SuiClientError::MissingKeyServerField {
            field_name: field_name.to_string(),
        };

        let value_field = parsed
            .fields
            .field_value("value")
            .ok_or_else(|| error_no_move_field("value"))?;

        let value_struct = match value_field {
            SuiMoveValue::Struct(value_struct) => value_struct,
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
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
            (
                SuiMoveValue::String(url),
                SuiMoveValue::String(name),
                SuiMoveValue::Vector(public_key_values),
            ) => {
                let public_key_bytes = parse_pk_bytes(key_server_id, public_key_values)?;
                let public_key = hex::encode(&public_key_bytes);

                (url, name, public_key)
            }
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
        };

        Ok(KeyServerInfo {
            object_id: ObjectID(key_server_id.into_bytes()),
            name,
            public_key,
            server_type: ServerType::Independent { url },
        })
    }

    async fn get_key_server_info_v2(
        &self,
        key_server_id: sui_types::base_types::ObjectID,
    ) -> Result<KeyServerInfo, SuiClientError> {
        let dynamic_field_name = DynamicFieldName {
            type_: TypeTag::U64,
            value: Value::String("2".to_string()),
        };

        let response = self
            .read_api()
            .get_dynamic_field_object(key_server_id, dynamic_field_name)
            .await?;

        let object_data =
            response
                .data
                .ok_or_else(|| SuiClientError::NoObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                })?;

        let content =
            object_data
                .content
                .ok_or_else(|| SuiClientError::NoObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                })?;

        let parsed = match content {
            SuiParsedData::MoveObject(obj) => obj,
            _ => {
                return Err(SuiClientError::InvalidObjectDataFromTheSuiRPC {
                    object_id: key_server_id,
                });
            }
        };

        let error_no_move_field = |field_name: &str| SuiClientError::MissingKeyServerField {
            field_name: field_name.to_string(),
        };

        let value_field = parsed
            .fields
            .field_value("value")
            .ok_or_else(|| error_no_move_field("value"))?;

        let value_struct = match value_field {
            SuiMoveValue::Struct(value_struct) => value_struct,
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
        };

        let name_value = value_struct
            .field_value("name")
            .ok_or_else(|| error_no_move_field("name"))?;

        let public_key_value = value_struct
            .field_value("pk")
            .ok_or_else(|| error_no_move_field("pk"))?;

        let server_type_value = value_struct
            .field_value("server_type")
            .ok_or_else(|| error_no_move_field("server_type"))?;

        let name = match name_value {
            SuiMoveValue::String(name) => name,
            // Some names (e.g. long hex committee names) may be deserialized as Address
            // due to serde untagged enum ordering.
            SuiMoveValue::Address(addr) => addr.to_string(),
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
        };

        let public_key = match public_key_value {
            SuiMoveValue::Vector(public_key_values) => {
                let public_key_bytes = parse_pk_bytes(key_server_id, public_key_values)?;
                hex::encode(&public_key_bytes)
            }
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
        };

        // Parse server_type to extract url, key server type, and optional committee info.
        // Due to serde untagged deserialization order, the RPC may return a Variant
        // (with an explicit "variant" field) or a Struct (when the SDK deserializes
        // the variant JSON as a struct, ignoring the extra "variant" key).
        let server_type = match server_type_value {
            SuiMoveValue::Variant(variant) => match variant.variant.as_str() {
                "Independent" => match variant.fields.get("url") {
                    Some(SuiMoveValue::String(url)) => ServerType::Independent { url: url.clone() },
                    _ => {
                        return Err(SuiClientError::MissingKeyServerField {
                            field_name: "server_type.Independent.url".to_string(),
                        });
                    }
                },
                "Committee" => parse_committee_from_fields(&variant.fields, key_server_id)?,
                _ => {
                    return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                        object_id: key_server_id,
                    });
                }
            },
            SuiMoveValue::Struct(ref s) => {
                // Fallback: serde may deserialize the enum variant as a Struct.
                // Independent variant has a "url" field, Committee does not.
                match s.field_value("url") {
                    Some(SuiMoveValue::String(url)) => ServerType::Independent { url },
                    _ => parse_committee_from_struct(s, key_server_id)?,
                }
            }
            _ => {
                return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                });
            }
        };

        Ok(KeyServerInfo {
            object_id: ObjectID(key_server_id.into_bytes()),
            name,
            public_key,
            server_type,
        })
    }
}

/// Parse `ServerType::Committee` from a `SuiMoveVariant`'s fields map.
fn parse_committee_from_fields(
    fields: &std::collections::BTreeMap<String, SuiMoveValue>,
    key_server_id: sui_types::base_types::ObjectID,
) -> Result<ServerType, SuiClientError> {
    let version = match fields.get("version") {
        Some(SuiMoveValue::Number(n)) => *n,
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.version".to_string(),
            });
        }
    };

    let threshold = match fields.get("threshold") {
        Some(SuiMoveValue::Number(n)) => {
            u16::try_from(*n).map_err(|_| SuiClientError::InvalidKeyServerDynamicFieldsType {
                object_id: key_server_id,
            })?
        }
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.threshold".to_string(),
            });
        }
    };

    let partial_key_servers = match fields.get("partial_key_servers") {
        Some(SuiMoveValue::Vector(entries)) => parse_partial_key_servers(entries, key_server_id)?,
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.partial_key_servers".to_string(),
            });
        }
    };

    Ok(ServerType::Committee {
        version,
        threshold,
        partial_key_servers,
    })
}

/// Parse `ServerType::Committee` from a `SuiMoveStruct` fallback.
fn parse_committee_from_struct(
    s: &sui_sdk::rpc_types::SuiMoveStruct,
    key_server_id: sui_types::base_types::ObjectID,
) -> Result<ServerType, SuiClientError> {
    let version = match s.field_value("version") {
        Some(SuiMoveValue::Number(n)) => n,
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.version".to_string(),
            });
        }
    };

    let threshold = match s.field_value("threshold") {
        Some(SuiMoveValue::Number(n)) => {
            u16::try_from(n).map_err(|_| SuiClientError::InvalidKeyServerDynamicFieldsType {
                object_id: key_server_id,
            })?
        }
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.threshold".to_string(),
            });
        }
    };

    let partial_key_servers = match s.field_value("partial_key_servers") {
        Some(SuiMoveValue::Vector(entries)) => parse_partial_key_servers(&entries, key_server_id)?,
        _ => {
            return Err(SuiClientError::MissingKeyServerField {
                field_name: "server_type.Committee.partial_key_servers".to_string(),
            });
        }
    };

    Ok(ServerType::Committee {
        version,
        threshold,
        partial_key_servers,
    })
}

/// Parse a vector of partial key server Move structs into `PartialKeyServer` entries.
fn parse_partial_key_servers(
    entries: &[SuiMoveValue],
    key_server_id: sui_types::base_types::ObjectID,
) -> Result<Vec<PartialKeyServer>, SuiClientError> {
    entries
        .iter()
        .map(|entry| {
            let s = match entry {
                SuiMoveValue::Struct(s) => s,
                _ => {
                    return Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                        object_id: key_server_id,
                    });
                }
            };

            let name = match s.field_value("name") {
                Some(SuiMoveValue::String(name)) => name,
                Some(SuiMoveValue::Address(addr)) => addr.to_string(),
                _ => String::new(),
            };

            let url = match s.field_value("url") {
                Some(SuiMoveValue::String(url)) => url,
                _ => {
                    return Err(SuiClientError::MissingKeyServerField {
                        field_name: "partial_key_server.url".to_string(),
                    });
                }
            };

            let partial_pk = match s.field_value("partial_pk") {
                Some(SuiMoveValue::Vector(pk_values)) => parse_pk_bytes(key_server_id, pk_values)?,
                _ => {
                    return Err(SuiClientError::MissingKeyServerField {
                        field_name: "partial_key_server.partial_pk".to_string(),
                    });
                }
            };

            let party_id = match s.field_value("party_id") {
                Some(SuiMoveValue::Number(n)) => u16::try_from(n).map_err(|_| {
                    SuiClientError::InvalidKeyServerDynamicFieldsType {
                        object_id: key_server_id,
                    }
                })?,
                _ => {
                    return Err(SuiClientError::MissingKeyServerField {
                        field_name: "partial_key_server.party_id".to_string(),
                    });
                }
            };

            Ok(PartialKeyServer {
                name,
                url,
                partial_pk,
                party_id,
            })
        })
        .collect()
}

fn parse_pk_bytes(
    key_server_id: sui_types::base_types::ObjectID,
    public_key_values: Vec<SuiMoveValue>,
) -> Result<Vec<u8>, SuiClientError> {
    public_key_values
        .into_iter()
        .map(|value| match value {
            SuiMoveValue::Number(byte) => match u8::try_from(byte) {
                Ok(byte) => Ok(byte),
                Err(_) => Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                    object_id: key_server_id,
                }),
            },
            _ => Err(SuiClientError::InvalidKeyServerDynamicFieldsType {
                object_id: key_server_id,
            }),
        })
        .collect::<Result<Vec<u8>, _>>()
}
