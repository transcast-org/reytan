use super::types::request;
use once_cell::sync::Lazy;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::{reqwest::header, ExtractionContext};
use serde::{Deserialize, Serialize};

pub static YOUTUBE_HOSTS_MAIN: Lazy<Vec<&str>> = Lazy::new(|| {
    vec![
        "youtube.com",
        "www.youtube.com",
        "m.youtube.com",
        "music.youtube.com",
        "youtubekids.com",
        "www.youtubekids.com",
    ]
});

pub static YOUTUBE_HOSTS_SHORT: Lazy<Vec<&str>> = Lazy::new(|| vec!["youtu.be", "y2u.be"]);

pub async fn innertube_request<T, S>(
    ctx: &ExtractionContext,
    client: &request::Client<'_>,
    endpoint: &str,
    json: S,
) -> Result<T>
where
    T: for<'a> Deserialize<'a>,
    S: Serialize,
{
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        client.user_agent.unwrap_or("okhttp/4.9.3").parse()?,
    );
    headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
    headers.insert(header::ORIGIN, format!("https://{}", client.host).parse()?);
    if let Some(client_id) = client.client_id {
        headers.insert("X-Youtube-Client-Name", client_id.into());
    }
    headers.insert(
        "X-Youtube-Client-Version",
        client.context.client_version.parse()?,
    );
    let resp = ctx
        .http
        .post(format!(
            "https://{}/youtubei/v1/{}?key={}",
            client.host, endpoint, client.api_key
        ))
        .json(&json)
        .headers(headers)
        .send()
        .await?
        .json::<T>()
        .await?;
    Ok(resp)
}
