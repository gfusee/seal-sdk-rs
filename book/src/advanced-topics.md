# Advanced Topics

This chapter shows how to plug in custom clients, caches, and transports, plus
how to pick the right feature flags and error strategy when you deploy the SDK.

## Custom Sui client

You can run `BaseSealClient` with a different version of the Sui SDK. Implement
the [`SuiClient`](../../src/sui_client.rs) trait for the client type you want to
use and update your `Cargo.toml` to point at that version.

The built-in implementation tries the V2 dynamic field first (key `2`) and falls
back to V1 (key `1`). V2 supports both independent and committee key servers via
a `ServerType` enum. For committee servers the on-chain URL is empty because
the aggregator URL is provided externally through `KeyServerConfig`.

If you write your own implementation, handle both versions to stay compatible
with older and newer key servers. See the default implementation in
`src/native_sui_sdk/client/sui_client.rs` for reference.

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
