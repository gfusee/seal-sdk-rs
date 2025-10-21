# Introduction

The `seal-sdk-rs` crate delivers a Sui framework–agnostic client for the Seal
encryption system. It puts developer experience first, exposing safe defaults
while keeping every layer — HTTP transport, Sui client, signing, and caching —
open for custom logic. Whether you want to plug in the fully-featured
`sui_sdk` stack or wire a lightweight experimental client, the SDK keeps the
abstractions narrow so you can tailor how requests are built, signed, cached
and executed.

Key characteristics:

- **Modular composition**: `BaseSealClient` is a generic over the HTTP client,
  Sui RPC adapter, caches, and error types, so you can mix and match
  implementations without forking the crate.
- **Pragmatic defaults**: the `SealClient` specializations pair
  `sui_sdk::SuiClient`, `reqwest`, and cache strategies (including `moka`) so
  most applications can get started immediately.
- **Parallel efficiency**: batch helpers (e.g. `encrypt_multiple_bytes`) reuse
  fetched metadata and execute remote calls concurrently when possible, keeping
  round trips to Mysten key servers short.
- **Ergonomic helpers**: conversion traits, generic object IDs/addresses, and
  BCS serializers keep data handling easy, while still allowing advanced users
  to manage serialization manually when required.