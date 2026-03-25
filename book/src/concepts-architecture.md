# Concepts & Architecture

This chapter explains how the SDK fits together: what powers `BaseSealClient`,
how the ready-made specializations behave, and which operational details matter
when you run the client in production.

## BaseSealClient

`BaseSealClient` (see `src/base_client.rs`) is the generic core. It exposes six
type parameters that let you decide which pieces to plug in:

- key-server info cache implementation
- derived-keys cache implementation
- Sui RPC error type
- Sui client implementation
- HTTP error type
- HTTP client implementation

You can supply any types that implement the required traits:

- [`SealCache`](../../src/cache.rs) defines a simple async cache API.
- [`SuiClient`](../../src/sui_client.rs) outlines the Sui RPC calls the client
  needs.
- [`HttpClient`](../../src/http_client.rs) asks for a single `post` method to
  talk to Seal key servers.

Because the generics stay abstract, you can swap components without editing the
rest of the crate—use a mock HTTP client in tests, replace the cache with a
shared service, or point to a different Sui SDK version.

## Specializations

`src/native_sui_sdk/client` offers ready-to-use type aliases:

- **`SealClient`** uses `sui_sdk::SuiClient`, `reqwest::Client`, and the `NoCache`
  adapters. The `client` and `native-sui-sdk` features enable it by default.
- **`SealClientLeakingCache`** adds `Arc<Mutex<HashMap<...>>>` caches. These
  caches never evict, so use them only for short-lived tools.
- **`SealClientMokaCache`** (behind the `moka-client` feature) relies on
  `moka::future::Cache`, giving you configurable eviction for long-lived
  services.

If you need something else, create your own alias that wires the right HTTP
client, Sui client, and caches into `BaseSealClient`.

## Caching strategies

Caching is optional but useful. The client can cache two kinds of data:

- key-server metadata fetched from Sui
- derived keys fetched from the Seal servers

`NoCache` skips caching. The `HashMap` and `moka` adapters show how to keep
results in memory. To integrate a different cache (Redis, a database, etc.),
implement `SealCache`.

## Session keys (JWT analogy)

`SessionKey` lives in `src/session_key.rs`. Instead of signing every decrypt
request with a wallet, you sign once to mint a short-lived key. Think of it like
a JWT:

1. A signer that implements `Signer` creates the session key.
2. During the TTL window, decrypt calls use that key without asking the wallet
   again.

Handle the session key like a bearer token: keep it safe in memory and drop it
when you no longer need it.

## Independent and committee key servers

Seal key servers come in two flavors. An **independent** server holds the full
master secret and serves key requests directly. A **committee** distributes the
secret across multiple participants via threshold cryptography; an **aggregator**
collects partial responses and returns the combined result.

The `KeyServerType` enum lets you tell the two apart programmatically:

```rust,ignore
use seal_sdk_rs::base_client::KeyServerType;

match info.key_server_type {
    KeyServerType::Independent => { /* server URL is in info.url */ }
    KeyServerType::Committee   => { /* needs an external aggregator URL */ }
}
```

You can retrieve a key server's metadata (name, URL, public key, type) at any
time with `get_key_server_info`:

```rust,ignore
let info = client.get_key_server_info(key_server_id).await?;
println!("name: {}, type: {:?}", info.name, info.key_server_type);
```

For encryption and decryption, the SDK handles both types transparently through
`KeyServerConfig`. Pass `aggregator_url: None` for independent servers and
`aggregator_url: Some(url)` for committees. During decryption, provide a
`HashMap<ObjectID, String>` mapping committee key server IDs to their aggregator
URLs. See the [Committee Servers](./committee-servers.md) chapter for details
and examples.

## Recovery keys and operational security

Every encrypt helper returns `(EncryptedObject, [u8; 32])`. The second value is
an emergency recovery key. Store it if you want a break-glass option when key
servers go offline. Drop it if you never want a single authority to decrypt all
payloads without the key-server quorum.

## Supported Sui SDKs and bridging types

Today you can choose between two Sui SDK families:

- `MystenLabs/sui` is mature but heavy. It pulls in a large dependency graph and
  build toolchain.
- `MystenLabs/sui-rust-sdk` is lightweight but still experimental.

`seal-sdk-rs` already bridges both worlds. `src/generic_types.rs` defines
`ObjectID` and `SuiAddress`, and the
`BCSSerializableProgrammableTransaction` trait hides differences between the SDKs.
Conversions run in both directions and all types support `serde`.

The built-in specializations (`SealClient`, `SealClientLeakingCache`,
`SealClientMokaCache`) currently target `MystenLabs/sui` and use JSON-RPC. gRPC
support is on the roadmap because the JSON-RPC endpoints have started their
phase-out. When the lightweight SDK stabilizes, new specializations can land
without changing the overall design.

## Feature flags overview

`Cargo.toml` exposes several public features:

- `default` = `client`, `native-tls`, `native-sui-sdk`
- `client` enables the HTTP layer (`reqwest` + `http`).
- `native-tls` switches `reqwest` to native TLS. Disable it if you want to opt
  into `rustls` manually.
- `native-sui-sdk` pulls in `sui_sdk`, `sui_types`, `sui_keys`, and
  `shared_crypto`, plus the Sui-specific adapters.
- `moka-client` adds the `moka` cache specialization.

Disable the defaults if you want to bring your own implementations and re-enable
only the pieces you need.
