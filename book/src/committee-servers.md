# Committee Servers

Seal supports two key server modes: **independent** and **committee**. This
chapter explains how they differ and how the SDK handles each.

## Independent vs committee

An **independent** key server is a single operator that holds the master secret
and responds to key requests directly. The on-chain `KeyServer` object stores
the server URL, and the SDK calls that URL to fetch derived keys.

A **committee** key server distributes the master secret across multiple
participants using threshold cryptography (MPC). No single member holds the
complete secret. An **aggregator** service coordinates key requests: it fans out
to individual members, collects partial responses until the threshold is met,
combines them, and returns the result. From the SDK's perspective, the
aggregator is the single endpoint to call.

## On-chain representation

Both modes use the same `KeyServer` Move object. The difference lies in the
`KeyServerV2` dynamic field's `ServerType` enum:

- `ServerType::Independent { url }` stores the server URL directly.
- `ServerType::Committee { version, threshold, partial_key_servers }` stores the
  threshold and a vector of `PartialKeyServer` entries (each with their own URL,
  partial public key, and party ID). There is no single URL on-chain for
  committees because the aggregator endpoint is provided by the caller.

The SDK reads the V2 dynamic field and falls back to V1 for older key servers.

## KeyServerConfig

`KeyServerConfig` wraps a key server's object ID with an optional aggregator URL:

```rust,ignore
use seal_sdk_rs::base_client::KeyServerConfig;

// Independent server: no aggregator URL needed.
let independent = KeyServerConfig::new(key_server_id, None);

// Committee server: provide the aggregator URL.
let committee = KeyServerConfig::new(
    key_server_id,
    Some("https://aggregator.example.com".to_string()),
);
```

During encryption, the aggregator URL has no effect; only the key server's
on-chain public key matters. During decryption, the SDK uses the aggregator URL
(when present) instead of the on-chain server URL to fetch derived keys.

## Encrypting with a committee

Encryption works the same way for both modes. The SDK fetches the public key
from the on-chain `KeyServer` object and encrypts locally:

```rust,no_run
use seal_sdk_rs::base_client::KeyServerConfig;
use seal_sdk_rs::error::SealClientError;
use seal_sdk_rs::generic_types::ObjectID;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use sui_sdk::SuiClientBuilder;

async fn encrypt_with_committee(
    package_id: ObjectID,
    key_server_id: ObjectID,
    aggregator_url: String,
) -> Result<seal_sdk_rs::crypto::EncryptedObject, SealClientError> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    let client = SealClient::new(sui_client);

    let key_server = KeyServerConfig::new(key_server_id, Some(aggregator_url));

    let (encrypted, _recovery_key) = client
        .encrypt_bytes(
            package_id,
            b"my_id".to_vec(),
            1,
            vec![key_server],
            b"secret data".to_vec(),
        )
        .await?;

    Ok(encrypted)
}
```

## Decrypting with a committee

During decryption, pass a map of key server object IDs to their aggregator URLs.
The SDK routes the key fetch request to the aggregator instead of the on-chain
URL:

```rust,no_run
use seal_sdk_rs::error::SealClientError;
use seal_sdk_rs::generic_types::ObjectID;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use std::collections::HashMap;
use sui_sdk::SuiClientBuilder;
use sui_sdk::wallet_context::WalletContext;
use sui_types::Identifier;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use std::str::FromStr;

async fn decrypt_with_committee(
    package_id: ObjectID,
    key_server_id: ObjectID,
    aggregator_url: String,
    encrypted: seal_sdk_rs::crypto::EncryptedObject,
) -> Result<Vec<u8>, SealClientError> {
    let sui_client = SuiClientBuilder::default()
        .build("https://fullnode.testnet.sui.io:443")
        .await?;
    let client = SealClient::new(sui_client);

    let mut wallet = WalletContext::new("<path to config>").unwrap();
    let session_key = SessionKey::new(package_id, 5, &mut wallet).await?;

    let mut builder = ProgrammableTransactionBuilder::new();
    let id_arg = builder.pure(b"my_id".to_vec())?;
    builder.programmable_move_call(
        package_id.into(),
        Identifier::from_str("wildcard")?,
        Identifier::from_str("seal_approve")?,
        vec![],
        vec![id_arg],
    );

    let aggregator_urls = HashMap::from([
        (key_server_id, aggregator_url),
    ]);

    let plaintext = client
        .decrypt_object_bytes(
            &bcs::to_bytes(&encrypted)?,
            builder.finish(),
            &session_key,
            aggregator_urls,
        )
        .await?;

    Ok(plaintext)
}
```

For independent servers, pass `HashMap::new()` (or omit the key server from the
map) and the SDK will use the on-chain URL as before.

## Mixing independent and committee servers

You can encrypt with multiple key servers where some are independent and others
are committee-based. During decryption, only include the committee servers in the
aggregator URL map; independent servers will automatically use their on-chain
URL:

```rust,ignore
let aggregator_urls = HashMap::from([
    (committee_key_server_id, "https://aggregator.example.com".to_string()),
    // independent_key_server_id is NOT in the map, so its on-chain URL is used.
]);

let plaintext = client
    .decrypt_object_bytes(&encrypted_bytes, ptb, &session_key, aggregator_urls)
    .await?;
```

## Error handling

When the aggregator is unreachable or returns an error, the SDK treats it the
same as a failed independent server: the response is excluded from the threshold
count. If too few servers respond, decryption fails with
`SealClientError::InsufficientKeys`.
