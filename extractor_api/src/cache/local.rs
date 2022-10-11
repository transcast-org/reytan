use std::{fs, io::ErrorKind, path::PathBuf};

use anyhow::Result;
use async_trait::async_trait;

use super::api::MapAPI;

#[derive(Clone)]
pub struct LocalCache {
    base_location: PathBuf,
}

impl LocalCache {
    pub fn from_location(base_location: PathBuf) -> Self {
        LocalCache { base_location }
    }
}

#[async_trait]
impl MapAPI for LocalCache {
    #[cfg(target_os = "linux")]
    fn new() -> Self {
        LocalCache {
            base_location: std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(std::env::var("HOME").unwrap()).join(".cache"))
                .join("reytan"),
        }
    }

    async fn get(&self, pool: &str, key: &str) -> Result<Option<Vec<u8>>> {
        match fs::read(self.base_location.join(pool).join(key)) {
            Ok(c) => Ok(Some(c)),
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn set(&self, pool: &str, key: &str, data: &[u8]) -> Result<()> {
        fs::create_dir_all(self.base_location.join(pool))?;
        fs::write(self.base_location.join(pool).join(key), data).map_err(anyhow::Error::from)
    }

    async fn has(&self, pool: &str, key: &str) -> Result<bool> {
        self.base_location
            .join(pool)
            .join(key)
            .try_exists()
            .map_err(anyhow::Error::from)
    }

    async fn delete(&self, pool: &str, key: &str) -> Result<()> {
        fs::remove_file(self.base_location.join(pool).join(key)).map_err(anyhow::Error::from)
    }
}
