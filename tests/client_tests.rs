use crate::utils::setup::setup;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use std::ops::DerefMut;
use std::str::FromStr;
use sui_sdk::SuiClientBuilder;
use sui_types::Identifier;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;

pub mod utils;

#[tokio::test]
async fn test_encrypt_decrypt_bytes_single_server() -> anyhow::Result<()> {
    let arc_setup = setup().await?;
    let mut setup_guard = arc_setup.lock_unchecked();
    let setup = setup_guard.deref_mut().as_mut().unwrap();

    let sui_client = SuiClientBuilder::default().build(&setup.rpc_url).await?;

    let seal_client = SealClient::new(sui_client);

    let data_to_encrypt = vec![0u8, 1, 2, 3];
    let data_id = vec![6u8];

    let encrypted = seal_client
        .encrypt_bytes(
            setup.approve_package_id,
            data_id.clone(),
            1,
            vec![setup.key_server_object_id],
            data_to_encrypt.clone(),
        )
        .await?;

    let mut approve_builder = ProgrammableTransactionBuilder::new();
    let id_arg = approve_builder.pure(data_id)?;

    _ = approve_builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("wildcard")?,
        Identifier::from_str("seal_approve")?,
        vec![],
        vec![id_arg],
    );

    let ptb = approve_builder.finish();

    let session_key = SessionKey::new(
        setup.approve_package_id,
        1,
        &mut setup.approve_package_deployer,
    )
    .await?;

    let decrypted = seal_client
        .decrypt_object_bytes(&bcs::to_bytes(&encrypted)?, ptb, &session_key)
        .await?;

    assert_eq!(decrypted, data_to_encrypt);

    Ok(())
}

#[tokio::test]
async fn test_encrypt_decrypt_u64_single_server() -> anyhow::Result<()> {
    let arc_setup = setup().await?;
    let mut setup_guard = arc_setup.lock_unchecked();
    let setup = setup_guard.deref_mut().as_mut().unwrap();

    let sui_client = SuiClientBuilder::default().build(&setup.rpc_url).await?;

    let seal_client = SealClient::new(sui_client);

    let data_to_encrypt = 17u64;
    let data_id = vec![6u8];

    let encrypted = seal_client
        .encrypt(
            setup.approve_package_id,
            data_id.clone(),
            1,
            vec![setup.key_server_object_id],
            data_to_encrypt,
        )
        .await?;

    let mut approve_builder = ProgrammableTransactionBuilder::new();
    let id_arg = approve_builder.pure(data_id)?;

    _ = approve_builder.programmable_move_call(
        setup.approve_package_id.into(),
        Identifier::from_str("wildcard").unwrap(),
        Identifier::from_str("seal_approve").unwrap(),
        vec![],
        vec![id_arg],
    );

    let ptb = approve_builder.finish();

    let session_key = SessionKey::new(
        setup.approve_package_id,
        1,
        &mut setup.approve_package_deployer,
    )
    .await?;

    let decrypted: u64 = seal_client
        .decrypt_object(&bcs::to_bytes(&encrypted)?, ptb, &session_key)
        .await?;

    assert_eq!(decrypted, data_to_encrypt);

    Ok(())
}
