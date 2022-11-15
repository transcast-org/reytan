use anyhow::{anyhow, Result};
use http_types::headers;
use serde::Deserialize;
use sys_locale::get_locale;

use crate::cache::api::{CacheAPI, CacheImplementation, MapAPI};
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use crate::cache::local::LocalCache;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use crate::cache::stub::StubCache;

#[derive(Clone)]
pub struct ExtractionContext {
    pub http: surf::Client,
    pub locales: Vec<String>,
    pub cache: CacheAPI,
}

impl ExtractionContext {
    pub fn new() -> ExtractionContext {
        let locale = get_locale()
            .filter(|l| l != "c" && l != "C")
            .unwrap_or_else(|| "en-US".to_string());

        let locales = if locale.len() > 2 {
            vec![locale.clone(), locale[0..2].to_string()]
        } else {
            vec![locale]
        };

        ExtractionContext {
            http: build_http(&locales),
            locales,
            // TODO: get actual cache implementations for other platforms as possible
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            cache: CacheAPI::new(CacheImplementation::Stub(StubCache::new())),
            #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
            cache: CacheAPI::new(CacheImplementation::Local(LocalCache::new())),
        }
    }

    pub fn new_with_locale(locales: Vec<String>) -> ExtractionContext {
        ExtractionContext {
            http: build_http(&locales),
            locales,
            cache: CacheAPI::new(CacheImplementation::Local(LocalCache::new())),
        }
    }

    pub async fn send_request(
        &self,
        _resource_name: &str,
        request: impl Into<surf::Request>,
    ) -> Result<surf::Response> {
        self.http.send(request).await.map_err(|e| anyhow!(e))
    }

    pub async fn get_body(
        &self,
        resource_name: &str,
        request: impl Into<surf::Request>,
    ) -> Result<String> {
        self.send_request(resource_name, request)
            .await?
            .body_string()
            .await
            .map_err(|e| anyhow!(e))
    }

    pub async fn get_json<T>(
        &self,
        resource_name: &str,
        request: impl Into<surf::Request>,
    ) -> Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        self.send_request(resource_name, request)
            .await?
            .body_json()
            .await
            .map_err(|e| anyhow!(e))
    }
}

pub fn build_http(locales: &Vec<String>) -> surf::Client {
    surf::Config::new()
        .add_header(headers::USER_AGENT, "okhttp/4.9.3")
        .unwrap()
        .add_header(
            headers::ACCEPT_LANGUAGE,
            locales
                .into_iter()
                .enumerate()
                .map(|(i, l)| {
                    if i != 0 {
                        format!("{l};q={}", 1.0 - (i as f32 / 10.0))
                    } else {
                        l.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        )
        .unwrap()
        .try_into()
        .unwrap()
}
