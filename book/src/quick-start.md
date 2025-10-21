# Quick Start

This walkthrough assumes you are using the crate with its default feature set,
which wires in the `SealClient` specialization (wrapping `sui_sdk::SuiClient`,
`reqwest`, and no-op caches).

## Install

Add the crate to your project:

```toml
[dependencies]
seal-sdk-rs = "0.0.1"
```

> **Info:** The examples below construct a `WalletContext` from a standard Sui
> CLI config (`WalletContext::new("<path to the config file>")`).
> 
> This is convenient but not mandatory—any signer that implements the crate's `Signer`
> trait can power `SessionKey::new`, so you can bring your own signing logic if
> desired.

## Example setup

The snippets below follow the same flow as the integration tests but are pared
down for clarity. Both examples share the same inputs:

- A single key server identified by `setup.key_server_id`.
- A Seal package deployed at `setup.approve_package_id`.
- A wallet context able to sign personal messages using
  `WalletContext::new("<path to the config file>")`.

All helpers return `Result<_, SealClientError>`, letting the `?` operator bubble
any issue up.

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

We assume the package deployed at `setup.approve_package_id` exposes a `wildcard`
module with a `seal_approve` function that always authorizes the `my_id`
payload.

Using the `EncryptedObject` returned by `encrypt_message`, the snippet
below builds the approval transaction and recovers the original string.

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
