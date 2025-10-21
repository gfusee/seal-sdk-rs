use crate::base_client::{BaseSealClient, DerivedKeys, KeyServerInfo};
use crate::cache::NoCache;
use crate::cache_key::{DerivedKeyCacheKey, KeyServerInfoCacheKey};
use crate::http_client::HttpClient;
use crate::sui_client::SuiClient;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SealClient = BaseSealClient<
    NoCache<KeyServerInfoCacheKey, KeyServerInfo>,
    NoCache<DerivedKeyCacheKey, DerivedKeys>,
    <sui_sdk::SuiClient as SuiClient>::Error,
    sui_sdk::SuiClient,
    <Client as HttpClient>::PostError,
    Client,
>;

impl SealClient {
    pub fn new(sui_client: sui_sdk::SuiClient) -> SealClient {
        BaseSealClient::new_custom(().into(), ().into(), sui_client, Client::new())
    }
}

pub type SealClientLeakingCache = BaseSealClient<
    Arc<Mutex<HashMap<KeyServerInfoCacheKey, KeyServerInfo>>>,
    Arc<Mutex<HashMap<DerivedKeyCacheKey, DerivedKeys>>>,
    <sui_sdk::SuiClient as SuiClient>::Error,
    sui_sdk::SuiClient,
    <Client as HttpClient>::PostError,
    Client,
>;

impl SealClientLeakingCache {
    pub fn new(sui_client: sui_sdk::SuiClient) -> SealClientLeakingCache {
        BaseSealClient::new_custom(
            Default::default(),
            Default::default(),
            sui_client,
            Client::new(),
        )
    }
}

#[cfg(feature = "moka")]
pub mod moka {
    use crate::client::base_client::{BaseSealClient, DerivedKeys, KeyServerInfo};
    use crate::client::cache_key::{DerivedKeyCacheKey, KeyServerInfoCacheKey};
    use crate::client::http_client::HttpClient;
    use crate::client::sui_client::SuiClient;
    use moka::future::{Cache, CacheBuilder};
    use reqwest::Client;

    pub type SealClientMokaCache = BaseSealClient<
        Cache<KeyServerInfoCacheKey, KeyServerInfo>,
        Cache<DerivedKeyCacheKey, Vec<DerivedKeys>>,
        <sui_sdk::SuiClient as SuiClient>::Error,
        sui_sdk::SuiClient,
        <Client as HttpClient>::PostError,
        Client,
    >;

    impl SealClientMokaCache {
        pub fn new(
            sui_client: sui_sdk::SuiClient,
            key_server_cache_builder: CacheBuilder<
                KeyServerInfoCacheKey,
                KeyServerInfo,
                Cache<KeyServerInfoCacheKey, KeyServerInfo>,
            >,
            derived_keys_cache_builder: CacheBuilder<
                DerivedKeyCacheKey,
                Vec<DerivedKeys>,
                Cache<DerivedKeyCacheKey, Vec<DerivedKeys>>,
            >,
        ) -> SealClientMokaCache {
            BaseSealClient::new_custom(
                key_server_cache_builder.build(),
                derived_keys_cache_builder.build(),
                sui_client,
                Client::new(),
            )
        }
    }
}
