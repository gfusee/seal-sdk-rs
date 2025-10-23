# Quick Start

This guide uses the crate with the default features. In this mode the
`SealClient` specialization is active, combining `sui_sdk::SuiClient`,
`reqwest`, and the no-op cache adapters.

## Install

Add the crate to your project:

```toml
[dependencies]
seal-sdk-rs = { git = "https://github.com/gfusee/seal-sdk-rs", tag = "0.0.2" }
```

> **Info:** The examples build a `WalletContext` from a normal Sui CLI config
> (`WalletContext::new("<path to the config file>")`). This approach is easy, but
> not required. Any signer that implements the crate's `Signer` trait can call
> `SessionKey::new`, so you can bring your own signing logic if you prefer.

## Example setup

The snippets follow the flow of the integration tests with fewer moving parts.
Both examples rely on the same inputs:

- One key server identified by `setup.key_server_id`.
- A Seal package deployed at `setup.approve_package_id`.
- A wallet context that can sign personal messages.

All helpers return `Result<_, SealClientError>`, so the `?` operator propagates
any failure.

### Encrypting a string

```rust,no_run
use seal_sdk_rs::error::SealClientError;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use sui_sdk::SuiClientBuilder;

struct DemoSetup {
    approve_package_id: seal_sdk_rs::generic_types::ObjectID,
    key_server_id: seal_sdk_rs::generic_types::ObjectID,
}

async fn encrypt_message(
    setup: &DemoSetup,
) -> Result<seal_sdk_rs::crypto::EncryptedObject, SealClientError> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;

    let client = SealClient::new(sui_client);
    let (encrypted, recovery_key) = client
        .encrypt_bytes(
            setup.approve_package_id,
            b"my_id".to_vec(),
            1,
            vec![setup.key_server_id],
            b"hello from seal".to_vec(),
        )
        .await?;

    drop(recovery_key);
    Ok(encrypted)
}
```

### Decrypting the ciphertext

Assume the package at `setup.approve_package_id` contains a `wildcard` module
with a `seal_approve` function that approves requests for `my_id`. Use the
`EncryptedObject` returned by `encrypt_message` to rebuild the approval
transaction and recover the original string.

```rust,no_run
use seal_sdk_rs::error::SealClientError;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use sui_sdk::SuiClientBuilder;
use sui_sdk::wallet_context::WalletContext;
use sui_types::Identifier;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use std::str::FromStr;

struct DemoSetup {
    approve_package_id: seal_sdk_rs::generic_types::ObjectID,
    key_server_id: seal_sdk_rs::generic_types::ObjectID,
}

async fn decrypt_message(
    setup: &DemoSetup,
    encrypted: seal_sdk_rs::crypto::EncryptedObject,
) -> Result<(), SealClientError> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    let client = SealClient::new(sui_client);

    let mut wallet = WalletContext::new("<path to the config file>").unwrap();
    let session_key = SessionKey::new(
        setup.approve_package_id,
        5,
        &mut wallet,
    )
    .await?;

    let mut builder = ProgrammableTransactionBuilder::new();
    let id_arg = builder.pure(b"my_id".to_vec())?;
    builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("wildcard")?,
        Identifier::from_str("seal_approve")?,
        vec![],
        vec![id_arg],
    );
    let approve_ptb = builder.finish();

    let plaintext = client
        .decrypt_object_bytes(
            &bcs::to_bytes(&encrypted)?,
            approve_ptb,
            &session_key,
        )
        .await?;

    assert_eq!(plaintext, b"hello from seal");
    Ok(())
}
```
