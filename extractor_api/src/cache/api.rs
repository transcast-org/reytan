use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{local::LocalCache, stub::StubCache};

#[async_trait]
/// Trait for storing data for later re-use by the extractors.
/// The trait does not deserialize or serialize stuff, this is done
/// by CacheAPI, which is a wrapper over it.
///
/// Inspired by JS [Map API](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Map),
/// [keyv](https://keyv.js.org/), and youtube-dl/yt-dlp cache storage.
pub trait MapAPI {
    fn new() -> Self
    where
        Self: Sized;

    async fn get(self: &Self, pool: &str, key: &str) -> Result<Option<Vec<u8>>>;

    async fn set(self: &Self, pool: &str, key: &str, data: &[u8]) -> Result<()>;

    async fn has(self: &Self, pool: &str, key: &str) -> Result<bool>;

    async fn delete(self: &Self, pool: &str, key: &str) -> Result<()>;
}

#[derive(Clone)]
pub enum CacheImplementation {
    Local(LocalCache),
    Stub(StubCache),
}

#[derive(Clone)]
pub struct CacheAPI {
    map: CacheImplementation,
}
impl CacheAPI {
    pub fn new(map: CacheImplementation) -> CacheAPI {
        CacheAPI { map }
    }

    fn deserialize<T>(self: &Self, getr: Result<Option<Vec<u8>>>) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        match getr {
            Ok(Some(b)) => serde_json::from_slice(&b)
                .map(Some)
                .map_err(anyhow::Error::from),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get<T>(self: &Self, pool: &str, key: &str) -> Result<Option<T>>
    where
        T: for<'a> Deserialize<'a>,
    {
        match &self.map {
            CacheImplementation::Local(c) => self.deserialize(c.get(pool, key).await),
            CacheImplementation::Stub(c) => self.deserialize(c.get(pool, key).await),
        }
    }

    fn serialize<T>(self: &Self, data: &T) -> Result<Vec<u8>>
    where
        T: Serialize,
    {
        serde_json::to_vec(data).map_err(anyhow::Error::from)
    }

    pub async fn set<T>(self: &Self, pool: &str, key: &str, data: &T) -> Result<()>
    where
        T: Serialize,
    {
        match &self.map {
            CacheImplementation::Local(c) => c.set(pool, key, &self.serialize(data)?).await,
            CacheImplementation::Stub(c) => c.set(pool, key, &self.serialize(data)?).await,
        }
    }
}
