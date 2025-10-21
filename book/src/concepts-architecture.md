# Concepts & Architecture

This chapter explains the moving parts behind `seal-sdk-rs`: how `BaseSealClient`
is composed, what the specializations wire together, and the operational
considerations you should keep in mind when shipping to production.

## BaseSealClient

`BaseSealClient` (see `src/base_client.rs`) is the generic heart of the SDK. It
has six type parameters covering

- key-server info cache implementation
- derived-keys cache implementation
- Sui RPC error type
- Sui client implementation
- HTTP error type
- HTTP client implementation

Any type satisfying the relevant traits can be used. The default crate exports
traits for each layer:

- [`SealCache`](../../src/cache.rs) – async cache API with a single
  `try_get_with` method.
- [`SuiClient`](../../src/sui_client.rs) – minimal RPC surface required to fetch
  key-server metadata.
- [`HttpClient`](../../src/http_client.rs) – simple `POST` wrapper for talking to
  Seal key servers.

Because the generic signature never mentions concrete types, projects can swap
out parts as needed (e.g. use a mock HTTP client in tests or a custom cache with
size limits).

## Specializations

The crate provides several type aliases in `src/native_sui_sdk/client` that wire
commonly-used components together:

- **`SealClient`** – the default specialization using
  `sui_sdk::SuiClient`, `reqwest::Client`, and the `NoCache` cache. Enabled by
  default via the `client` and `native-sui-sdk` feature flags.
- **`SealClientLeakingCache`** – same as `SealClient` but with in-memory
  `HashMap` caches wrapped in `Arc<Mutex<_>>`. The caches never evict, so use it
  only for short-lived tooling.
- **`SealClientMokaCache`** – feature-gated behind `moka-client`. Uses
  `moka::future::Cache` for both caches, giving you configurable eviction logic
  for long-running services.

When customizing, you can also supply your own type alias, choosing whichever
Sui client, HTTP client, and caches make sense for your environment.

## Caching strategies

Caching is optional. The two cache traits exist to avoid repeated RPC lookups
and key fetches:

- The *key-server info cache* stores object metadata fetched via Sui RPC.
- The *derived-keys cache* stores the results of key reconstruction from the
  Seal key servers.

`NoCache` is the simplest adapter and simply forwards every miss. The `HashMap`
and `moka` adapters show how to layer in memory-backed caches when you want to
cache results within a process. You can plug in any other cache (redis, memcache
client, etc.) by implementing `SealCache`.

## Session keys (JWT analogy)

`SessionKey` lives in `src/session_key.rs`. Instead of forcing a wallet to sign
authentication payloads for every decryption request, the SDK lets you mint a
short-lived key that behaves like a JWT token:

1. The wallet signs a capability once (using any `Signer` implementation).
2. During the TTL window, decrypt calls present the session key to the key
   servers, avoiding repeated signatures.

Treat the session key like a bearer token: store it securely and drop it when it
expires or when you no longer need to issue decrypt calls.

## Recovery keys and operational security

Every encrypt helper returns a tuple `(EncryptedObject, [u8; 32])`. The second
value is an emergency recovery key that allows decryption even if key servers go
offline. Keep these keys somewhere safe if you want a break-glass option; drop
or wipe them immediately if you do *not* want any backdoor that bypasses key
server quorum.

## Supported Sui SDKs and bridging types

Two Sui SDK ecosystems exist today:

- The monorepo at `MystenLabs/sui`, which ships the mature, feature-rich client
  but pulls in a large dependency graph and build toolchain.
- The lighter `MystenLabs/sui-rust-sdk`, currently experimental but designed to
  be leaner.

`seal-sdk-rs` anticipates both worlds. Bridging types in `src/generic_types.rs`
(`ObjectID`, `SuiAddress`) and the
`BCSSerializableProgrammableTransaction` trait allow either SDK’s types to pass
through the same API surface. Conversions exist both ways and everything stays
`serde`-friendly.

Today’s built-in specializations (`SealClient`, `SealClientLeakingCache`, and
`SealClientMokaCache`) target the `MystenLabs/sui` SDK and speak JSON-RPC. gRPC
support is planned as the upstream JSON-RPC endpoints move deeper into their
deprecation cycle. Once the lightweight SDK stabilizes, new specializations can
slot in without architectural changes.

## Feature flags overview

`Cargo.toml` defines the following public flags:

- `default` = `client`, `native-tls`, `native-sui-sdk`
- `client` – enables `reqwest` + `http` abstractions, required for
  `SealClient`.
- `native-tls` – configures `reqwest` to use native TLS. Switch to `rustls` by
  disabling defaults and enabling the relevant `reqwest` feature manually.
- `native-sui-sdk` – pulls in `sui_sdk`, `sui_types`, `sui_keys`, and
  `shared_crypto`, plus Sui-specific adapters.
- `moka-client` – adds the `moka` cache specialization (`SealClientMokaCache`).

Disable defaults if you want to bring entirely custom implementations; re-enable
only the pieces you need.
