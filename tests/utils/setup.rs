// Copyright 2025 Quentin Diebold
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::anyhow;
use base64::Engine;
use reqwest::Client;
use seal_sdk_rs::generic_types::{ObjectID, SuiAddress};
use serde::Deserialize;
use serde_json::json;
use std::convert::TryInto;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use sui_json_rpc_types::SuiTransactionBlockEffectsAPI;
use sui_keys::keystore::{InMemKeystore, Keystore};
use sui_sdk::rpc_types::SuiTransactionBlockResponseOptions;
use sui_sdk::wallet_context::WalletContext;
use sui_sdk::{SUI_COIN_TYPE, SuiClientBuilder};
use sui_types::object::Owner;
use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use sui_types::transaction::TransactionData;
use testcontainers::core::{ContainerPort, ExecCommand};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::OnceCell;

pub const APPROVE_PACKAGE: [&str; 1] = [
    "oRzrCwYAAAAGAQACAwIFBQcEBwsWCCEgDEEHAAEAAAABAAEKAgAMc2VhbF9hcHByb3ZlCHdpbGRjYXJkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQAAQECAA==",
];

pub const SEAL_SERVER_COUNT: usize = 3;

pub struct Setup {
    pub rpc_url: String,
    pub approve_package_id: ObjectID,
    pub approve_package_deployer: WalletContext,
    pub localnet_container: ContainerAsync<GenericImage>,
    pub seal_instances: [SealInstance; SEAL_SERVER_COUNT],
    pub extra_seal_servers_count: usize,
}

impl Setup {
    pub async fn add_new_seal_servers<const N: usize>(
        &mut self,
    ) -> anyhow::Result<[SealInstance; N]> {
        let start_index = SEAL_SERVER_COUNT + self.extra_seal_servers_count + N;

        let mut result = Vec::with_capacity(N);
        for index in start_index..start_index + N {
            result.push(create_seal_instance(index).await?);
        }

        self.extra_seal_servers_count += N;

        Ok(result.try_into().ok().unwrap())
    }
}

pub struct SealInstance {
    pub seal_container: ContainerAsync<GenericImage>,
    pub key_server_id: ObjectID,
    pub seal_package_id: ObjectID,
    pub seal_server_url: String,
    pub public_key: [u8; 96],
}

#[derive(Deserialize, Debug)]
struct SealInfo {
    seal_package_id: String,
    #[serde(rename = "key_server_object_id")]
    key_server_package_id: String,
    public_key: String,
}

const LOCALNET_IMAGE_NAME: &str = "seal-sdk-rs-localnet";
const LOCALNET_IMAGE_TAG: &str = "latest";
const LOCALNET_CONTAINER_NAME: &str = "seal-sdk-rs-localnet";

const SEAL_SERVER_IMAGE_NAME: &str = "seal-sdk-rs-seal-server";
const SEAL_SERVER_IMAGE_TAG: &str = "latest";
const SEAL_SERVER_CONTAINER_NAME: &str = "seal-sdk-rs-seal-server";
const SEAL_SERVER_INTERNAL_PORT: u16 = 2024;

const DOCKER_NETWORK: &str = "seal-sdk-rs";

static SETUP: OnceCell<ArcSetup> = OnceCell::const_new();

#[derive(Clone)]
pub struct ArcSetup {
    inner: Arc<Mutex<Option<Setup>>>,
}

impl ArcSetup {
    pub fn lock_unchecked<'a>(&'a self) -> MutexGuard<'a, Option<Setup>> {
        self.inner.lock().unwrap()
    }
}

impl Drop for ArcSetup {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 2 {
            let setup = SETUP.get().unwrap().inner.lock().unwrap().take();

            drop(setup)
        }
    }
}

pub async fn setup() -> anyhow::Result<ArcSetup> {
    SETUP
        .get_or_try_init(|| async {
            let setup = init_setup().await?;
            let inner = Arc::new(Mutex::new(Some(setup)));

            anyhow::Result::Ok(ArcSetup { inner })
        })
        .await
        .cloned()
}

pub async fn init_setup() -> anyhow::Result<Setup> {
    let localnet = GenericImage::new(LOCALNET_IMAGE_NAME, LOCALNET_IMAGE_TAG)
        .with_exposed_port(ContainerPort::Tcp(9000))
        .with_exposed_port(ContainerPort::Tcp(9123))
        .with_network(DOCKER_NETWORK)
        .with_container_name(LOCALNET_CONTAINER_NAME)
        .start()
        .await?;

    let seal_instances_vec = {
        let mut instances = Vec::with_capacity(SEAL_SERVER_COUNT);
        for index in 0..SEAL_SERVER_COUNT {
            instances.push(create_seal_instance(index).await?);
        }
        instances
    };

    let seal_instances: [SealInstance; SEAL_SERVER_COUNT] =
        seal_instances_vec
            .try_into()
            .map_err(|instances: Vec<SealInstance>| {
                anyhow!(
                    "Expected {SEAL_SERVER_COUNT} seal instances, got {}",
                    instances.len()
                )
            })?;

    let rpc_external_port = localnet.get_host_port_ipv4(9000).await?;
    let rpc_external_url = format!("http://localhost:{rpc_external_port}");

    let faucet_external_port = localnet.get_host_port_ipv4(9123).await?;
    let faucet_external_url = format!("http://localhost:{faucet_external_port}");

    let mut deployer_wallet = setup_deployment_wallet();
    let deployer_address = deployer_wallet.active_address()?.into();

    faucet(&faucet_external_url, &deployer_address).await?;

    let approve_package_id =
        deploy_approve_package(&rpc_external_url, &mut deployer_wallet).await?;

    let setup = Setup {
        rpc_url: rpc_external_url,
        approve_package_id,
        approve_package_deployer: deployer_wallet,
        localnet_container: localnet,
        seal_instances,
        extra_seal_servers_count: 0,
    };

    Ok(setup)
}

async fn create_seal_instance(index: usize) -> anyhow::Result<SealInstance> {
    let free_port = find_free_port().await?;

    let seal_server_external_url = format!("http://localhost:{free_port}");
    let container_name = format!("{SEAL_SERVER_CONTAINER_NAME}-{index}");

    let seal = GenericImage::new(SEAL_SERVER_IMAGE_NAME, SEAL_SERVER_IMAGE_TAG)
        .with_mapped_port(free_port, ContainerPort::Tcp(SEAL_SERVER_INTERNAL_PORT))
        .with_network(DOCKER_NETWORK)
        .with_container_name(container_name)
        .with_env_var("NODE_URL", format!("http://{LOCALNET_CONTAINER_NAME}:9000"))
        .with_env_var(
            "FAUCET_URL",
            format!("http://{LOCALNET_CONTAINER_NAME}:9123"),
        )
        .with_env_var("SEAL_SERVER_URL", seal_server_external_url.clone())
        .start()
        .await?;

    let seal_server_external_port = seal.get_host_port_ipv4(SEAL_SERVER_INTERNAL_PORT).await?;

    wait_for_seal_server(seal_server_external_port).await;

    let mut result = seal
        .exec(ExecCommand::new(["cat", "/shared/seal.json"]))
        .await?;

    let mut stdout = String::new();
    let mut reader = result.stdout();

    reader.read_to_string(&mut stdout).await?;

    let info: SealInfo =
        serde_json::from_str(&stdout).map_err(|_| anyhow!("Cannot get seal info"))?;

    let SealInfo {
        seal_package_id,
        key_server_package_id,
        public_key,
    } = info;

    let public_key_hex = public_key.strip_prefix("0x").unwrap_or(&public_key);
    let public_key_bytes = hex::decode(public_key_hex)?;
    let public_key: [u8; 96] = public_key_bytes.try_into().map_err(|bytes: Vec<u8>| {
        anyhow!(
            "Invalid public key length: expected 96 bytes, got {}",
            bytes.len()
        )
    })?;

    Ok(SealInstance {
        seal_container: seal,
        key_server_id: key_server_package_id.parse()?,
        seal_package_id: seal_package_id.parse()?,
        seal_server_url: seal_server_external_url,
        public_key,
    })
}

async fn wait_for_seal_server(port: u16) {
    let client = Client::new();
    let url = format!("http://localhost:{port}");

    loop {
        match client.get(&url).send().await {
            Ok(_) => {
                println!("Server is up!");
                break;
            }
            _ => {
                println!("Waiting for server...");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

fn setup_deployment_wallet() -> WalletContext {
    let random_keystore = Keystore::InMem(InMemKeystore::new_insecure_for_tests(1));
    WalletContext::new_for_tests(random_keystore, None, None)
}

async fn faucet(faucet_url: &str, wallet_address: &SuiAddress) -> Result<(), reqwest::Error> {
    let url = format!("{}/v2/gas", faucet_url);
    let payload = json!({
        "FixedAmountRequest": {
            "recipient": wallet_address.to_string(),
        }
    });

    let client = Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    println!("Status: {}", status);
    println!("Response: {}", text);

    Ok(())
}

async fn deploy_approve_package(
    rpc_url: &str,
    wallet: &mut WalletContext,
) -> anyhow::Result<ObjectID> {
    let client = SuiClientBuilder::default().build(rpc_url).await?;

    let package_bytes = APPROVE_PACKAGE.map(|module_base64| {
        base64::engine::general_purpose::STANDARD
            .decode(module_base64)
            .unwrap()
    });

    let mut builder = ProgrammableTransactionBuilder::new();

    builder.publish_immutable(package_bytes.to_vec(), vec![]);

    let ptb = builder.finish();

    let sender = wallet.active_address()?;

    let gas_payment = client
        .coin_read_api()
        .get_coins(sender, Some(SUI_COIN_TYPE.to_string()), None, None)
        .await?
        .data
        .into_iter()
        .next()
        .unwrap();

    let tx_data = TransactionData::new_programmable(
        sender,
        vec![gas_payment.object_ref()],
        ptb,
        10000000000,
        1000,
    );

    let tx = wallet.sign_transaction(&tx_data).await;

    let result = client
        .quorum_driver_api()
        .execute_transaction_block(
            tx,
            SuiTransactionBlockResponseOptions::new().with_effects(),
            None,
        )
        .await?;

    let package_id: ObjectID = result
        .effects
        .unwrap()
        .created()
        .iter()
        .find(|e| matches!(e.owner, Owner::Immutable))
        .unwrap()
        .reference
        .object_id
        .into();

    Ok(package_id)
}

async fn find_free_port() -> anyhow::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    Ok(port)
}
