use anyhow::Result;
use ratmom::http::header;
use ratmom::{AsyncBody, AsyncReadResponseExt, HttpClient, HttpClientBuilder, Request, Response};
use serde::Deserialize;
use sys_locale::get_locale;

use crate::cache::api::{CacheAPI, CacheImplementation, MapAPI};
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
use crate::cache::local::LocalCache;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use crate::cache::stub::StubCache;

#[derive(Clone)]
pub struct ExtractionContext {
    pub http: HttpClient,
    pub locales: Vec<String>,
    pub cache: CacheAPI,
}

impl ExtractionContext {
    pub fn new() -> Result<ExtractionContext> {
        let locale = get_locale()
            .filter(|l| l != "c" && l != "C")
            .unwrap_or_else(|| "en-US".to_string());

        let locales = if locale.len() > 2 {
            vec![locale.clone(), locale[0..2].to_string()]
        } else {
            vec![locale]
        };

        Ok(ExtractionContext {
            http: build_http(&locales)?,
            locales,
            // TODO: get actual cache implementations for other platforms as possible
            #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
            cache: CacheAPI::new(CacheImplementation::Stub(StubCache::new())),
            #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
            cache: CacheAPI::new(CacheImplementation::Local(LocalCache::new())),
        })
    }

    pub fn new_with_locale(locales: Vec<String>) -> Result<ExtractionContext> {
        Ok(ExtractionContext {
            http: build_http(&locales)?,
            locales,
            cache: CacheAPI::new(CacheImplementation::Local(LocalCache::new())),
        })
    }

    pub async fn send_request<'a, Q>(
        &self,
        _resource_name: &str,
        request: Request<Q>,
    ) -> Result<Response<AsyncBody>>
    where
        Q: Into<AsyncBody>,
    {
        Ok(self.http.send_async(request).await?)
    }

    pub async fn get_body<'a, Q>(&self, resource_name: &str, request: Request<Q>) -> Result<String>
    where
        Q: Into<AsyncBody>,
    {
        Ok(self
            .send_request(resource_name, request)
            .await?
            .text()
            .await?)
    }

    pub async fn get_json<Q, A>(&self, resource_name: &str, request: Request<Q>) -> Result<A>
    where
        Q: Into<AsyncBody>,
        A: for<'a> Deserialize<'a> + Unpin,
    {
        Ok(self
            .send_request(resource_name, request)
            .await?
            .json()
            .await?)
    }
}

pub fn build_http(locales: &Vec<String>) -> Result<HttpClient> {
    Ok(HttpClientBuilder::new()
        .default_header(header::USER_AGENT, "okhttp/4.9.3")
        .default_header(
            header::ACCEPT_LANGUAGE,
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
        .build()?)
}
