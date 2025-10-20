use std::collections::HashMap;
use std::sync::Arc;
use core::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use async_trait::async_trait;
use tokio::sync::Mutex;

#[async_trait]
pub trait SealCache: Send + Sync {
    type Key;
    type Value;

    async fn try_get_with<Fut, Error>(&self, key: Self::Key, init: Fut) -> Result<Self::Value, Arc<Error>>
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

    async fn try_get_with<Fut, Error>(&self, _key: Self::Key, init: Fut) -> Result<Self::Value, Arc<Error>>
    where
        Fut: Future<Output=Result<Self::Value, Error>> + Send,
        Error: Send + Sync + 'static
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
    async fn try_get_with<Fut, Error>(&self, key: Self::Key, init: Fut) -> Result<Self::Value, Arc<Error>>
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
                },
                Err(err) => {
                    Err(Arc::new(err))
                }
            }
        }
    }
}

#[cfg(feature = "moka")]
mod moka {
    use std::hash::Hash;
    use std::sync::Arc;
    use async_trait::async_trait;
    use crate::client::cache::SealCache;

    #[async_trait]
    impl<Key, Value> SealCache for moka::future::Cache<Key, Value>
    where
        Key: Eq + Hash + Send + Sync + 'static,
        Value: Clone + Send + Sync + 'static,
    {
        type Key = Key;
        type Value = Value;

        async fn try_get_with<Fut, Error>(&self, key: Self::Key, init: Fut) -> Result<Self::Value, Arc<Error>>
        where
            Fut: Future<Output=Result<Self::Value, Error>> + Send,
            Error: Send + Sync + 'static,
        {
            moka::future::Cache::try_get_with(self, key, init).await
        }
    }
}