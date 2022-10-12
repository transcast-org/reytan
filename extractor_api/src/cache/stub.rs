use anyhow::Result;
use async_trait::async_trait;

use super::api::MapAPI;

#[derive(Clone)]
pub struct StubCache {}

#[async_trait]
impl MapAPI for StubCache {
    fn new() -> Self {
        StubCache {}
    }

    async fn get(&self, _pool: &str, _key: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn set(&self, _pool: &str, _key: &str, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    async fn has(&self, _pool: &str, _key: &str) -> Result<bool> {
        Ok(false)
    }

    async fn delete(&self, _pool: &str, _key: &str) -> Result<()> {
        Ok(())
    }
}
