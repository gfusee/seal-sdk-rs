use crate::cache::SealCache;
use crate::cache_key::{DerivedKeyCacheKey, KeyServerInfoCacheKey};
use crate::error::SealClientError;
use crate::generic_types::{BCSSerializableProgrammableTransaction, ObjectID};
use crate::http_client::HttpClient;
use crate::session_key::SessionKey;
use crate::sui_client::SuiClient;
use seal_crypto::{
    seal_encrypt, EncryptionInput, IBEPublicKeys,
};
use fastcrypto::groups::FromTrustedByteArray;
use futures::future::join_all;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use fastcrypto::groups::bls12381::G2Element;
use crate::crypto::{seal_decrypt_all_objects, EncryptedObject, FetchKeyRequest, FetchKeyResponse};

/// Key server object layout containing object id, name, url, and public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyServerInfo {
    pub object_id: ObjectID,
    pub name: String,
    pub url: String,
    pub public_key: String,
}

pub type DerivedKeys = (ObjectID, FetchKeyResponse);

#[derive(Clone)]
pub struct BaseSealClient<KeyServerInfoCache, DerivedKeysCache, SuiError, Sui, HttpError, Http>
where
    KeyServerInfoCache: SealCache<Key = KeyServerInfoCacheKey, Value = KeyServerInfo>,
    DerivedKeysCache: SealCache<Key = DerivedKeyCacheKey, Value = Vec<DerivedKeys>>,
    SealClientError: From<SuiError>,
    SuiError: Send + Sync + Display + 'static,
    Sui: SuiClient<Error = SuiError>,
    SealClientError: From<HttpError>,
    Http: HttpClient<PostError = HttpError>,
{
    key_server_info_cache: KeyServerInfoCache,
    derived_key_cache: DerivedKeysCache,
    sui_client: Sui,
    http_client: Http,
}

impl<KeyServerInfoCache, DerivedKeysCache, SuiError, Sui, HttpError, Http> BaseSealClient<KeyServerInfoCache, DerivedKeysCache, SuiError, Sui, HttpError, Http>
where
    KeyServerInfoCache: SealCache<Key = KeyServerInfoCacheKey, Value = KeyServerInfo>,
    DerivedKeysCache: SealCache<Key = DerivedKeyCacheKey, Value = Vec<DerivedKeys>>,
    SealClientError: From<SuiError>,
    SuiError: Send + Sync + Display + 'static,
    Sui: SuiClient<Error = SuiError>,
    SealClientError: From<HttpError>,
    Http: HttpClient<PostError = HttpError>,
{
    pub fn new_custom(
        key_server_info_cache: KeyServerInfoCache,
        derived_key_cache: DerivedKeysCache,
        sui_client: Sui,
        http_client: Http
    ) -> Self {
        BaseSealClient {
            key_server_info_cache,
            derived_key_cache,
            sui_client,
            http_client,
        }
    }

    pub async fn encrypt_bytes<ID1, ID2>(
        &self,
        package_id: ID1,
        id: Vec<u8>,
        threshold: u8,
        key_servers: Vec<ID2>,
        data: Vec<u8>,
    ) -> Result<EncryptedObject, SealClientError>
    where
        ObjectID: From<ID1>,
        ObjectID: From<ID2>,
    {
        let package_id: ObjectID = package_id.into();
        let key_servers = key_servers
            .into_iter()
            .map(ObjectID::from)
            .collect::<Vec<_>>();

        let key_server_info = self.fetch_key_server_info(key_servers.clone()).await?;
        let public_keys_g2 = key_server_info
            .iter()
            .map(|info| self.decode_public_key(info))
            .collect::<Result<_, _>>()?;

        let public_keys = IBEPublicKeys::BonehFranklinBLS12381(public_keys_g2);

        let key_servers = key_servers
            .into_iter()
            .map(|e| e.into())
            .collect();

        let result = seal_encrypt(
            package_id.0.into(),
            id,
            key_servers,
            &public_keys,
            threshold,
            EncryptionInput::Aes256Gcm { data, aad: None },
        )?;

        Ok(result.0.into())
    }

    #[allow(dead_code)]
    pub async fn key_server_info(
        &self,
        key_server_ids: Vec<ObjectID>,
    ) -> Result<Vec<KeyServerInfo>, SealClientError> {
        self.fetch_key_server_info(key_server_ids).await
    }

    pub async fn decrypt_object<T, PTB>(
        &self,
        encrypted_object_data: &[u8],
        approve_transaction_data: PTB,
        session_key: &SessionKey,
    ) -> Result<T, SealClientError>
    where
        T: DeserializeOwned,
        PTB: BCSSerializableProgrammableTransaction,
    {
        let encrypted_object = bcs::from_bytes::<EncryptedObject>(encrypted_object_data)?;

        let service_ids: Vec<ObjectID> = encrypted_object
            .services
            .iter()
            .map(|(id, _)| (*id).into())
            .collect();

        let key_server_info = self.fetch_key_server_info(service_ids).await?;
        let servers_public_keys_map = key_server_info
            .iter()
            .map(|info| {
                Ok::<_, SealClientError>((
                    info.object_id.into(),
                    self.decode_public_key(info)?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect::<HashMap<_, _>>();

        let (signed_request, enc_secret) = session_key.get_fetch_key_request(approve_transaction_data.to_bcs_bytes()?)?;

        let derived_keys = self
            .fetch_derived_keys(
                signed_request,
                key_server_info,
                encrypted_object.threshold,
            )
            .await?
            .into_iter()
            .map(|derived_key| (derived_key.0.into(), derived_key.1))
            .collect::<Vec<_>>();

        let encrypted_objects = vec![encrypted_object];
        let decrypted_result = seal_decrypt_all_objects(
            &enc_secret,
            &derived_keys,
            encrypted_objects,
            &servers_public_keys_map,
        )?
            .into_iter()
            .next()
            .ok_or_else(|| SealClientError::MissingDecryptedObject)?;

        Ok(bcs::from_bytes::<T>(&decrypted_result)?)
    }

    async fn fetch_key_server_info(
        &self,
        key_server_ids: Vec<ObjectID>,
    ) -> Result<Vec<KeyServerInfo>, SealClientError> {
        let mut key_server_info_futures = vec![];
        for key_server_id in key_server_ids {
            let cache_key = KeyServerInfoCacheKey::new(key_server_id);

            let future = async move {
                self.key_server_info_cache
                    .try_get_with(
                        cache_key,
                        self.sui_client.get_key_server_info(key_server_id.0)
                    )
                    .await
                    .map_err(unwrap_cache_error)
            };

            key_server_info_futures.push(future);
        }

        join_all(key_server_info_futures)
            .await
            .into_iter()
            .collect::<Result<_, _>>()
            .map_err(Into::into)
    }

    async fn fetch_derived_keys(
        &self,
        request: FetchKeyRequest,
        key_servers_info: Vec<KeyServerInfo>,
        threshold: u8,
    ) -> Result<Vec<DerivedKeys>, SealClientError> {
        let request_json = request.to_json_string()?;

        let server_ids: Vec<ObjectID> =
            key_servers_info.iter().map(|info| info.object_id).collect();

        let cache_key = DerivedKeyCacheKey::new(
            request_json.clone().into_bytes(),
            server_ids,
            threshold
        );

        let cache_future = async {
            let mut seal_responses: Vec<DerivedKeys> = Vec::new();
            for server in key_servers_info.iter() {
                let mut headers = HashMap::new();

                headers.insert("Client-Sdk-Type".to_string(), "rust".to_string());
                headers.insert("Client-Sdk-Version".to_string(), "1.0.0".to_string());
                headers.insert("Content-Type".to_string(), "application/json".to_string());

                let url = format!("{}/v1/fetch_key", server.url);
                let response = self
                    .http_client
                    .post(
                        &url,
                        headers,
                        request_json.clone()
                    )
                    .await?;

                if !response.is_success() {
                    return Err(SealClientError::ErrorWhileFetchingDerivedKeys {
                        url,
                        status: response.status,
                        response: response.text
                    });
                }

                let response: FetchKeyResponse = serde_json::from_str(&response.text)?;

                seal_responses.push((server.object_id, response));

                if seal_responses.len() >= threshold as usize {
                    break;
                }
            }

            let seal_responses_len = seal_responses.len();

            if seal_responses_len < threshold as usize {
                return Err(SealClientError::InsufficientKeys {
                    received: seal_responses_len,
                    threshold,
                });
            }

            Ok(seal_responses)
        };

        self.derived_key_cache
            .try_get_with(
                cache_key,
                cache_future
            )
            .await
            .map_err(unwrap_cache_error)
    }

    fn decode_public_key(&self, info: &KeyServerInfo) -> Result<G2Element, SealClientError> {
        let bytes = hex::decode(&info.public_key)?;

        let array: [u8; 96] = bytes.as_slice().try_into().map_err(|_| {
            SealClientError::InvalidPublicKey {
                public_key: info.public_key.clone(),
                reason: "Invalid length.".to_string()
            }
        })?;

        Ok(G2Element::from_trusted_byte_array(&array)?)
    }
}

fn unwrap_cache_error<T>(err: Arc<T>) -> SealClientError
where
    T: Display,
    SealClientError: From<T>
{
    Arc::try_unwrap(err)
        .map(Into::into)
        .unwrap_or_else(|wrapped_error| {
            SealClientError::CannotUnwrapTypedError {
                error_message: wrapped_error.to_string(),
            }
        })
}
