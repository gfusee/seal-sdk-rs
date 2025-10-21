# Advanced Topics

This chapter shows how to plug in custom clients, caches, and transports, plus
how to pick the right feature flags and error strategy when you deploy the SDK.

## Custom Sui client

You can run `BaseSealClient` with a different version of the Sui SDK. Implement
the [`SuiClient`](../../src/sui_client.rs) trait for the client type you want to
use and update your `Cargo.toml` to point at that version. The snippet below is
based on the default implementation for `sui_sdk::SuiClient` and can serve as a
starting point:

```rust,no_run
use seal_sdk_rs::base_client::KeyServerInfo;
use seal_sdk_rs::generic_types::ObjectID;
use seal_sdk_rs::sui_client::SuiClient;
use async_trait::async_trait;
use serde_json::Value;
use sui_sdk::rpc_types::{SuiMoveValue, SuiParsedData};
use sui_types::TypeTag;
use sui_types::dynamic_field::DynamicFieldName;

#[derive(Debug, thiserror::Error)]
pub enum CustomSuiError {
    #[error("Sui SDK error: {0}")]
    SuiSdk(#[from] sui_sdk::error::Error),
    #[error("Missing object data for {object_id}")]
    Missing { object_id: sui_types::base_types::ObjectID },
    #[error("Unexpected data shape for {object_id}")]
    Invalid { object_id: sui_types::base_types::ObjectID },
}

#[async_trait]
impl SuiClient for sui_sdk::SuiClient {
    type Error = CustomSuiError;

    async fn get_key_server_info(
        &self,
        key_server_id: [u8; 32],
    ) -> Result<KeyServerInfo, Self::Error> {
        let key_server_id = sui_types::base_types::ObjectID::new(key_server_id);
        let response = self
            .read_api()
            .get_dynamic_field_object(
                key_server_id,
                DynamicFieldName {
                    type_: TypeTag::U64,
                    value: Value::String("1".to_string()),
                },
            )
            .await?;

        let object_data = response.data.ok_or(CustomSuiError::Missing { object_id: key_server_id })?;
        let content = object_data.content.ok_or(CustomSuiError::Missing { object_id: key_server_id })?;

        let parsed = match content {
            SuiParsedData::MoveObject(obj) => obj,
            _ => return Err(CustomSuiError::Invalid { object_id: key_server_id }),
        };

        let value_struct = match parsed.fields.field_value("value") {
            Some(SuiMoveValue::Struct(s)) => s,
            _ => return Err(CustomSuiError::Invalid { object_id: key_server_id }),
        };

        let url = match value_struct.field_value("url") {
            Some(SuiMoveValue::String(url)) => url,
            _ => return Err(CustomSuiError::Invalid { object_id: key_server_id }),
        };

        let name = match value_struct.field_value("name") {
            Some(SuiMoveValue::String(name)) => name,
            _ => return Err(CustomSuiError::Invalid { object_id: key_server_id }),
        };

        let public_key_bytes = match value_struct.field_value("pk") {
            Some(SuiMoveValue::Vector(bytes)) => bytes
                .into_iter()
                .map(|value| match value {
                    SuiMoveValue::Number(byte) => u8::try_from(byte).map_err(|_| CustomSuiError::Invalid { object_id: key_server_id }),
                    _ => Err(CustomSuiError::Invalid { object_id: key_server_id }),
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Err(CustomSuiError::Invalid { object_id: key_server_id }),
        };

        Ok(KeyServerInfo {
            object_id: ObjectID(key_server_id.into_bytes()),
            name,
            url,
            public_key: hex::encode(public_key_bytes),
        })
    }
}
```

Compile `seal-sdk-rs` against your chosen dependency version and the new
implementation becomes active.

## Custom HTTP client

[`HttpClient`](../../src/http_client.rs) defines a single method. Implement it
for your preferred transport (hyper, surf, a custom blocking client, etc.):

```rust
use std::collections::HashMap;

async fn post<S: ToString + Send + Sync>(
    &self,
    url: &str,
    headers: HashMap<String, String>,
    body: S,
) -> Result<PostResponse, Self::PostError>;
```

Seal key servers only expect HTTP `POST` requests, so you do not need anything
else.

## Custom caching

To plug in your own cache, implement [`SealCache`](../../src/cache.rs). The key
method, `try_get_with`, either returns a cached value or runs the provided
future to populate the cache:

```rust
use std::future::Future;
use std::sync::Arc;

async fn try_get_with<Fut, Error>(
    &self,
    key: Self::Key,
    init: Fut,
) -> Result<Self::Value, Arc<Error>>
where
    Fut: Future<Output = Result<Self::Value, Error>> + Send,
    Error: Send + Sync + 'static;
```

Whenever possible, add request coalescing so you collapse duplicate misses into
one in-flight future. This reduces unnecessary parallel calls, keeps you away
from Seal server rate limits, and lightens the load on Sui RPC endpoints.

## Error handling strategies

Public helpers return `Result<_, SealClientError>`. Examples and tests sometimes
use `anyhow::Result<_>`. Both styles support the `?` operator, so you can bubble
errors up or wrap them in your own enums for richer diagnostics.

## Deployment considerations

- **Feature gating**: Disable the default feature set when you want a custom
  stack, then re-enable only what you need (for example,
  `--no-default-features --features client,moka-client`).
- **Parallel behavior**: Encrypt and decrypt helpers send requests in parallel.
  Keep an eye on quotas for key servers and Sui RPC. Caches plus coalescing help
  control the traffic.
- **Recovery keys**: Decide whether to store the `[u8; 32]` recovery key that
  encrypt helpers return. Dropping it removes a potential backdoor. Storing it
  gives you an emergency path if key servers become unavailable.
