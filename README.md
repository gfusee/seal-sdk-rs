# seal-sdk-rs

A developer-friendly Rust client for the Mysten Seal encryption system. The
crate works with any Sui setup: keep the default stack (`SealClient`) or swap in
your own HTTP transport, Sui client, signer, and cache.

## Highlights

- Modular `BaseSealClient` so you can replace each layer.
- Ready-to-use `SealClient` specializations for `sui_sdk::SuiClient` and
  `reqwest` (with optional `moka` caching).
- Session keys act like short-lived JWTs, so wallets do not sign every request.
- Encryption helpers return recovery keys for break-glass scenarios.
- Bridging types let you use both `MystenLabs/sui` and `sui-rust-sdk` APIs.

## Install

```toml
[dependencies]
seal-sdk-rs = "0.0.1"
```

## Quick start

```rust,no_run
use seal_sdk_rs::error::SealClientError;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use sui_sdk::SuiClientBuilder;
use sui_sdk::wallet_context::WalletContext;
use sui_types::Identifier;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use std::str::FromStr;

async fn encrypt_and_decrypt(
    package_id: seal_sdk_rs::generic_types::ObjectID,
    key_server_id: seal_sdk_rs::generic_types::ObjectID,
) -> Result<(), SealClientError> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    let client = SealClient::new(sui_client);

    let (encrypted, recovery_key) = client
        .encrypt_bytes(
            package_id,
            b"demo-id".to_vec(),
            1,
            vec![key_server_id],
            b"hello seal".to_vec(),
        )
        .await?;
    drop(recovery_key);

    let mut wallet = WalletContext::new("<path to the config file>").unwrap();
    let session_key = SessionKey::new(package_id, 5, &mut wallet).await?;

    let mut builder = ProgrammableTransactionBuilder::new();
    let id_arg = builder.pure(b"demo-id".to_vec())?;
    builder.programmable_move_call(
        package_id.into(),
        Identifier::from_str("wildcard")?,
        Identifier::from_str("seal_approve")?,
        vec![],
        vec![id_arg],
    );

    let plaintext = client
        .decrypt_object_bytes(&bcs::to_bytes(&encrypted)?, builder.finish(), &session_key)
        .await?;

    assert_eq!(plaintext, b"hello seal");
    Ok(())
}
```

For a full walkthrough, open the docs in `book/` (run `mdbook serve book`).

## Concepts at a glance

- `BaseSealClient` accepts custom HTTP, Sui, cache, and error types.
- `SealClientLeakingCache` and `SealClientMokaCache` show different cache
  strategies.
- `SessionKey` lets wallets sign once per TTL window (JWT analogy).
- Encrypt helpers return `(EncryptedObject, [u8; 32])` so you can decide what to
  do with the recovery key.
- Bridging traits (`ObjectID`, `SuiAddress`,
  `BCSSerializableProgrammableTransaction`) let you mix Sui SDK ecosystems.

## Bringing your own components

- Implement [`SuiClient`](src/sui_client.rs) to target a different Sui SDK
  version.
- Implement [`HttpClient`](src/http_client.rs) for a custom transport (only a
  `post` method is required).
- Implement [`SealCache`](src/cache.rs) for your own cache (add request
  coalescing if you can).
- Implement [`Signer`](src/signer.rs) when you want to mint session keys without
  `WalletContext`.

## Feature flags

| Feature         | Description                                              |
|-----------------|----------------------------------------------------------|
| `client`        | Enables `reqwest` + HTTP abstractions. Included by default. |
| `native-tls`    | Uses native TLS with `reqwest`. Included by default.         |
| `native-sui-sdk`| Pulls in the `MystenLabs/sui` crates and adapters.       |
| `moka-client`   | Adds the `SealClientMokaCache` specialization.           |

Disable the default features if you plan to provide your own stack.

## Documentation

- mdBook guide: run `mdbook serve book`
- API docs: `cargo doc --open`

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
