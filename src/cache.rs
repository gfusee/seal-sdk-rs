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

use async_trait::async_trait;
use core::future::Future;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Minimal async-friendly cache abstraction used by [`BaseSealClient`](crate::base_client::BaseSealClient).
///
/// Implementors can decide how to memoize expensive fetches executed by the seal client.
/// This crate ships a "do nothing" implementation via [`NoCache`] and, when the `moka`
/// feature is enabled (via the `moka-client` feature flag), a
/// [Moka-powered](https://docs.rs/moka) adapter in
/// [`native_sui_sdk::client::seal_client::moka`](crate::native_sui_sdk::client::seal_client::moka).
/// The `SealClient` specializations demonstrate how these caches are wired into higher-level
/// clients.
#[async_trait]
pub trait SealCache: Send + Sync {
    type Key;
    type Value;

    async fn try_get_with<Fut, Error>(
        &self,
        key: Self::Key,
        init: Fut,
    ) -> Result<Self::Value, Arc<Error>>
    where
        Fut: Future<Output = Result<Self::Value, Error>> + Send,
        Error: Send + Sync + 'static;
}

#[derive(Copy, Clone, Debug)]
pub struct NoCache<Key, Value> {
    _phantom_key: PhantomData<Key>,
    _phantom_value: PhantomData<Value>,
}

impl<Key, Value> Default for NoCache<Key, Value> {
    fn default() -> Self {
        NoCache {
            _phantom_key: PhantomData,
            _phantom_value: PhantomData,
        }
    }
}

impl<Key, Value> From<()> for NoCache<Key, Value> {
    fn from(_: ()) -> Self {
        Self {
            _phantom_key: PhantomData,
            _phantom_value: PhantomData,
        }
    }
}

#[async_trait]
impl<Key: Send + Sync, Value: Send + Sync> SealCache for NoCache<Key, Value> {
    type Key = Key;
    type Value = Value;

    async fn try_get_with<Fut, Error>(
        &self,
        _key: Self::Key,
        init: Fut,
    ) -> Result<Self::Value, Arc<Error>>
    where
        Fut: Future<Output = Result<Self::Value, Error>> + Send,
        Error: Send + Sync + 'static,
    {
        init.await.map_err(Arc::new)
    }
}

#[async_trait]
impl<Key, Value> SealCache for Arc<Mutex<HashMap<Key, Value>>>
where
    Key: Eq + Hash + Send,
    Value: Clone + Send,
{
    type Key = Key;
    type Value = Value;

    // Simple implementation that doesn't perform any kind of request coalescing
    async fn try_get_with<Fut, Error>(
        &self,
        key: Self::Key,
        init: Fut,
    ) -> Result<Self::Value, Arc<Error>>
    where
        Fut: Future<Output = Result<Self::Value, Error>> + Send,
        Error: Send + Sync + 'static,
    {
        let cached_value = {
            let cache = self.lock().await;
            cache.get(&key).cloned()
        };

        if let Some(value) = cached_value {
            Ok(value.clone())
        } else {
            let value = init.await;

            match value {
                Ok(value) => {
                    {
                        let mut cache = self.lock().await;
                        cache.insert(key, value.clone());
                    }

                    Ok(value)
                }
                Err(err) => Err(Arc::new(err)),
            }
        }
    }
}

#[cfg(feature = "moka")]
mod moka {
    use crate::client::cache::SealCache;
    use async_trait::async_trait;
    use std::hash::Hash;
    use std::sync::Arc;

    #[async_trait]
    impl<Key, Value> SealCache for moka::future::Cache<Key, Value>
    where
        Key: Eq + Hash + Send + Sync + 'static,
        Value: Clone + Send + Sync + 'static,
    {
        type Key = Key;
        type Value = Value;

        async fn try_get_with<Fut, Error>(
            &self,
            key: Self::Key,
            init: Fut,
        ) -> Result<Self::Value, Arc<Error>>
        where
            Fut: Future<Output = Result<Self::Value, Error>> + Send,
            Error: Send + Sync + 'static,
        {
            moka::future::Cache::try_get_with(self, key, init).await
        }
    }
}
