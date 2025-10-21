use crate::base_client::{BaseSealClient, DerivedKeys, KeyServerInfo};
use crate::cache::NoCache;
use crate::cache_key::{DerivedKeyCacheKey, KeyServerInfoCacheKey};
use crate::http_client::HttpClient;
use crate::sui_client::SuiClient;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// High-level client preconfigured for the crate's default feature set.
///
/// `SealClient` wires the generic [`BaseSealClient`] with the concrete
/// `sui_sdk::SuiClient`, `reqwest::Client`, and the no-op cache implementations
/// bundled with this crate. It is the variant enabled by default features and
/// is what almost every developer should reach for. When full control over the
/// underlying HTTP client, Sui client, or cache layers is required, prefer the
/// raw [`BaseSealClient`] type instead. See the integration tests for examples
/// of how this specialization is used in practice. Encryption helpers return a
/// tuple of the encrypted payload plus an emergency recovery key—drop the key if
/// you do not want a single authority to retain the power to decrypt every
/// payload without the key servers.
///
/// # Examples
///
/// ```rust,no_run
/// use seal_sdk_rs::generic_types::ObjectID;
/// use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClient;
/// use sui_sdk::SuiClientBuilder;
/// use std::str::FromStr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let sui_client = SuiClientBuilder::default()
///         .build("https://fullnode.testnet.sui.io:443")
///         .await?;
///
///     let seal_client = SealClient::new(sui_client);
///
///     let key_server_id =
///         ObjectID::from_str("0x6f4c8bead1dcbef4b880d1b845a70d820ee4da8b36805b97d93ef3e829ae8b55")?;
///
///     let (encrypted, recovery_key) = seal_client
///         .encrypt_bytes(
///             ObjectID::from_str(
///                 "0xf5f3a4e1d0c19a43b2c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90123456",
///             )?,
///             b"demo-data".to_vec(),
///             1,
///             vec![key_server_id],
///             b"secret payload".to_vec(),
///         )
///         .await?;
///
///     drop(recovery_key); // Discard to avoid retaining an authority-level backdoor.
///     println!("Encrypted object: {:?}", encrypted);
///     Ok(())
/// }
/// ```
///
/// # Dealing with SDK versions
///
/// This specialization pins the versions of `sui_sdk::SuiClient`, `reqwest::Client`,
/// and the cache implementations it wires in. If your application needs to align with
/// different dependency versions, disable the crate's default features in your
/// `Cargo.toml`, supply your own implementations of the required traits, and
/// instantiate [`BaseSealClient::new_custom`].
///
/// For reference, the default adapters ship in
/// [`native_sui_sdk::client::sui_client`](crate::native_sui_sdk::client::sui_client) (Sui RPC),
/// [`reqwest::client`](crate::reqwest::client) (HTTP transport), and
/// [`cache`](crate::cache) (cache implementations).
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

/// [`SealClient`] variant that layers simple in-memory `HashMap` caches.
///
/// This convenience type is handy for short-lived tooling or tests, but it keeps
/// every cached entry for the lifetime of the process. Avoid using it in
/// long-running services because the unbounded growth can lead to out-of-memory
/// failures. Its encryption helpers return the encrypted payload together with
/// the recovery key—discard the key if you do not want a standalone authority to
/// decrypt everything. Prefer the eviction-friendly
/// [`SealClientMokaCache`](crate::native_sui_sdk::client::seal_client::moka::SealClientMokaCache)
/// when you need caching in production scenarios.
///
/// # Examples
///
/// ```rust,no_run
/// use seal_sdk_rs::generic_types::ObjectID;
/// use seal_sdk_rs::native_sui_sdk::client::seal_client::SealClientLeakingCache;
/// use sui_sdk::SuiClientBuilder;
/// use std::str::FromStr;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let sui_client = SuiClientBuilder::default()
///         .build("https://fullnode.testnet.sui.io:443")
///         .await?;
///
///     let seal_client = SealClientLeakingCache::new(sui_client);
///
///     let key_server_id =
///         ObjectID::from_str("0x6f4c8bead1dcbef4b880d1b845a70d820ee4da8b36805b97d93ef3e829ae8b55")?;
///
///     let (encrypted, recovery_key) = seal_client
///         .encrypt_bytes(
///             ObjectID::from_str(
///                 "0xf5f3a4e1d0c19a43b2c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90123456",
///             )?,
///             b"demo-data".to_vec(),
///             1,
///             vec![key_server_id],
///             b"secret payload".to_vec(),
///         )
///         .await?;
///
///     drop(recovery_key); // Discard to avoid retaining an authority-level backdoor.
///     println!("Encrypted object: {:?}", encrypted);
///     Ok(())
/// }
/// ```
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

    /// [`SealClient`] specialization backed by [`moka`](https://docs.rs/moka) caches.
    ///
    /// Requires the crate's `moka` feature. Compared with
    /// [`SealClientLeakingCache`](crate::native_sui_sdk::client::seal_client::SealClientLeakingCache),
    /// the Moka-backed caches allow you to configure capacity and eviction,
    /// making this variant a better fit for long-lived services. Like the other
    /// specializations, encryption calls return both the encrypted payload and a
    /// recovery key—drop the key to avoid creating a single-party backdoor.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use moka::future::CacheBuilder;
    /// use seal_sdk_rs::generic_types::ObjectID;
    /// use seal_sdk_rs::native_sui_sdk::client::seal_client::moka::SealClientMokaCache;
    /// use sui_sdk::SuiClientBuilder;
    /// use std::str::FromStr;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let sui_client = SuiClientBuilder::default()
    ///         .build("https://fullnode.testnet.sui.io:443")
    ///         .await?;
    ///
    ///     let key_server_cache_builder = CacheBuilder::new(1_000);
    ///     let derived_keys_cache_builder = CacheBuilder::new(1_000);
    ///
    ///     let seal_client = SealClientMokaCache::new(
    ///         sui_client,
    ///         key_server_cache_builder,
    ///         derived_keys_cache_builder,
    ///     );
    ///
    ///     let key_server_id =
    ///         ObjectID::from_str("0x6f4c8bead1dcbef4b880d1b845a70d820ee4da8b36805b97d93ef3e829ae8b55")?;
    ///
    ///     let (encrypted, recovery_key) = seal_client
    ///         .encrypt_bytes(
    ///             ObjectID::from_str(
    ///                 "0xf5f3a4e1d0c19a43b2c7d8e9f0a1b2c3d4e5f60718293a4b5c6d7e8f90123456",
    ///             )?,
    ///             b"demo-data".to_vec(),
    ///             1,
    ///             vec![key_server_id],
    ///             b"secret payload".to_vec(),
    ///         )
    ///         .await?;
    ///
    ///     drop(recovery_key); // Discard to avoid retaining an authority-level backdoor.
    ///     println!("Encrypted object: {:?}", encrypted);
    ///     Ok(())
    /// }
    /// ```
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
