use super::types::request;
use once_cell::sync::Lazy;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::{header, ExtractionContext, Request};
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
    resource_name: &str,
    client: &request::Client<'_>,
    endpoint: &str,
    json: S,
) -> Result<T>
where
    T: for<'a> Deserialize<'a> + Unpin,
    S: Serialize,
{
    let mut request = Request::post(format!(
        "https://{}/youtubei/v1/{}?key={}",
        client.host, endpoint, client.api_key
    ))
    .header(
        header::USER_AGENT,
        client.user_agent.unwrap_or("okhttp/4.9.3"),
    )
    .header(header::ORIGIN, format!("https://{}", client.host))
    .header(header::CONTENT_TYPE, "application/json")
    .header("X-Youtube-Client-Version", client.context.client_version);
    if let Some(client_id) = client.client_id {
        request = request.header("X-Youtube-Client-Name", client_id.to_string());
    }
    let resp = ctx
        .get_json::<String, T>(resource_name, request.body(serde_json::to_string(&json)?)?)
        .await?;
    Ok(resp)
}
