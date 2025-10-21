# Advanced Topics

Deep dive into customization options beyond the default `SealClient`. The
architecture lets you bring a different version of the Sui SDK, your own HTTP
transport, custom caches, and bespoke error wrapping, while keeping the ergonomic
public surface.

## Custom Sui client

Use `BaseSealClient` with a different Sui SDK revision by re-implementing the
[`SuiClient`](../../src/sui_client.rs) trait. As an example, the default crate
implements the trait for `sui_sdk::SuiClient`. You can copy that implementation
and change the dependency version in `Cargo.toml` to match your fork:

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

Compile `seal-sdk-rs` with your modified dependency version and the new
implementation takes effect.

## Custom HTTP client

[`HttpClient`](../../src/http_client.rs) only requires one async method:

```rust
use std::collections::HashMap;

async fn post<S: ToString + Send + Sync>(
    &self,
    url: &str,
    headers: HashMap<String, String>,
    body: S,
) -> Result<PostResponse, Self::PostError>;
```

Implement this trait for your transport of choice (hyper, surf, reqwest with
rustls, etc.) and use it when constructing a `BaseSealClient`. No other HTTP
methods are needed—the Seal key servers only expect `POST` requests.

## Custom caching

Caches implement [`SealCache`](../../src/cache.rs). The core method is
`try_get_with`, which either returns an existing value or executes the provided
future to populate the cache:

```rust
use std::sync::Arc;
use std::future::Future;

async fn try_get_with<Fut, Error>(
    &self,
    key: Self::Key,
    init: Fut,
) -> Result<Self::Value, Arc<Error>>
where
    Fut: Future<Output = Result<Self::Value, Error>> + Send,
    Error: Send + Sync + 'static;
```

When writing your own cache layer, add request coalescing if possible so multiple
identical misses take the same in-flight future. That helps avoid hammering the
Seal HTTP endpoints or the Sui RPC and keeps you under rate limits.

## Error handling strategies

All public async helpers return either `Result<_, SealClientError>` or an
`anyhow::Result<_>` inside tests/examples. In both cases the `?` operator is
available, so you can bubble up errors directly or wrap them in your own enums
if you prefer more specific handling.

## Deployment considerations

- **Feature gating**: disable the `default` feature set and re-enable only what
  you need (e.g. `--no-default-features --features client,moka-client` when you
  want `SealClientMokaCache`).
- **Parallel behavior**: encrypt/decrypt helpers issue concurrent requests, so
  monitor key-server and RPC quotas. Custom caches and coalescing strategies can
  help keep load stable.
- **Recovery keys**: decide up front whether to store the recovery key returned
  from encrypt helpers. Dropping it eliminates an authority-level backdoor;
  storing it gives you an emergency escape hatch if key servers go offline.
