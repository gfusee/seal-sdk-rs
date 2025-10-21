# Introduction

`seal-sdk-rs` gives you a Seal client that works with any Sui framework. The
crate focuses on developer experience. It ships safe defaults, but you can
replace every layer: HTTP transport, Sui client, signer, and cache. Use the
full `sui_sdk` stack or a lighter experimental client. The API stays simple so
you can control how requests are built, signed, cached, and executed.

Key features:

- **Modular design**: `BaseSealClient` accepts generic types for the HTTP
  client, Sui RPC adapter, caches, and error types. You can mix and match
  implementations without forking the crate.
- **Helpful defaults**: The provided `SealClient` variants combine
  `sui_sdk::SuiClient`, `reqwest`, and different cache strategies (including
  `moka`) so most projects can start quickly.
- **Parallel performance**: Batch helpers such as `encrypt_multiple_bytes`
  reuse fetched metadata and run remote calls in parallel when possible. This
  keeps round trips to Mysten key servers short.
- **Friendly helpers**: Conversion traits, generic object IDs/addresses, and
  BCS serializers simplify data handling. Advanced users can still manage
  serialization manually when they need to.
