use crate::utils::setup::setup;
use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
use seal_sdk_rs::session_key::SessionKey;
use std::ops::DerefMut;
use std::str::FromStr;
use anyhow::bail;
use reqwest::Client;
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
            vec![setup.seal_instances[0].key_server_id.clone()],
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
            vec![setup.seal_instances[0].key_server_id.clone()],
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

#[tokio::test]
async fn test_encrypt_decrypt_bytes_three_servers() -> anyhow::Result<()> {
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
            3,
            setup.seal_instances.iter().map(|e| e.key_server_id).collect(),
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
async fn test_encrypt_decrypt_bytes_three_servers_threshold_two_one_crash() -> anyhow::Result<()> {
    let arc_setup = setup().await?;
    let mut setup_guard = arc_setup.lock_unchecked();
    let setup = setup_guard.deref_mut().as_mut().unwrap();
    let [seal_instance_that_will_crash] = setup
        .add_new_seal_servers::<1>()
        .await?;

    let sui_client = SuiClientBuilder::default().build(&setup.rpc_url).await?;

    let seal_client = SealClient::new(sui_client);

    let data_to_encrypt = vec![0u8, 1, 2, 3];
    let data_id = vec![6u8];

    let mut seal_instances_that_wont_crash_ids_iter = setup.seal_instances
        .iter()
        .map(|e| e.key_server_id);

    let seal_instances = vec![
        seal_instances_that_wont_crash_ids_iter.next().unwrap(),
        seal_instances_that_wont_crash_ids_iter.next().unwrap(),
        seal_instance_that_will_crash.key_server_id
    ];

    let encrypted = seal_client
        .encrypt_bytes(
            setup.approve_package_id,
            data_id.clone(),
            2,
            seal_instances,
            data_to_encrypt.clone(),
        )
        .await?;

    let crashed_server_url = seal_instance_that_will_crash.seal_server_url.clone();
    drop(seal_instance_that_will_crash);
    wait_for_seal_server_to_be_off(&crashed_server_url).await;

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
async fn test_encrypt_decrypt_bytes_three_servers_threshold_three_one_crash() -> anyhow::Result<()> {
    let arc_setup = setup().await?;
    let mut setup_guard = arc_setup.lock_unchecked();
    let setup = setup_guard.deref_mut().as_mut().unwrap();
    let [seal_instance_that_will_crash] = setup
        .add_new_seal_servers::<1>()
        .await?;

    let sui_client = SuiClientBuilder::default().build(&setup.rpc_url).await?;

    let seal_client = SealClient::new(sui_client);

    let data_to_encrypt = vec![0u8, 1, 2, 3];
    let data_id = vec![6u8];

    let mut seal_instances_that_wont_crash_ids_iter = setup.seal_instances
        .iter()
        .map(|e| e.key_server_id);

    let seal_instances = vec![
        seal_instances_that_wont_crash_ids_iter.next().unwrap(),
        seal_instances_that_wont_crash_ids_iter.next().unwrap(),
        seal_instance_that_will_crash.key_server_id
    ];

    let encrypted = seal_client
        .encrypt_bytes(
            setup.approve_package_id,
            data_id.clone(),
            3,
            seal_instances,
            data_to_encrypt.clone(),
        )
        .await?;

    let crashed_server_url = seal_instance_that_will_crash.seal_server_url.clone();
    drop(seal_instance_that_will_crash);
    wait_for_seal_server_to_be_off(&crashed_server_url).await;

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

    let decrypted_result = seal_client
        .decrypt_object_bytes(&bcs::to_bytes(&encrypted)?, ptb, &session_key)
        .await;

    match decrypted_result {
        Ok(_) => {
            bail!("Should not succeed!")
        },
        Err(_) => {}
    }

    Ok(())
}

async fn wait_for_seal_server_to_be_off(url: &str) {
    let client = Client::new();

    loop {
        match client.get(url).send().await {
            Ok(_) => {
                println!("Server is up...");
            }
            Err(_) => {
                println!("Server is off!");
                break;
            }
        }
    }
}