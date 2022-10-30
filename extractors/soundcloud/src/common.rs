use nipper::Document;
use once_cell::sync::Lazy;
use qstring::QString;
use regex::Regex;
use reytan_extractor_api::anyhow::{anyhow, bail, Result};
use reytan_extractor_api::surf::Response;
use reytan_extractor_api::url::Url;
use reytan_extractor_api::ExtractionContext;
use serde::Deserialize;

pub static SOUNDCLOUD_USER_DOMAINS: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["soundcloud.com", "www.soundcloud.com", "m.soundcloud.com"]);

pub static SOUNDCLOUD_API_DOMAINS: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["api.soundcloud.com", "api-v2.soundcloud.com"]);

static SOUNDCLOUD_WEB_URL: Lazy<Url> = Lazy::new(|| Url::parse("https://soundcloud.com/").unwrap());

static WEB_JS_CLIENT_ID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[{,]\s*["']?client_id["']?\s*:\s*["']([a-zA-Z0-9]{32})["']"#).unwrap()
});

// must be a separate non-async function for nipper reasons
fn get_script_urls(webpage: &str) -> Vec<Url> {
    Document::from(webpage)
        .select("script[src]")
        .iter()
        .flat_map(|s| s.attr("src"))
        .flat_map(|u| SOUNDCLOUD_WEB_URL.join(&u.to_string()))
        .filter(|u| u.host_str() == Some("a-v2.sndcdn.com") && u.path().starts_with("/assets/"))
        .rev()
        .collect()
}

async fn extract_client_id(ctx: &ExtractionContext) -> Result<String> {
    let webpage = ctx
        .get_body("webpage", ctx.http.get(SOUNDCLOUD_WEB_URL.clone()))
        .await?;
    for script_url in get_script_urls(&webpage) {
        let script = ctx.get_body("js script", ctx.http.get(script_url)).await?;
        if let Some(capture) = WEB_JS_CLIENT_ID.captures(&script) {
            return Ok(capture.get(1).unwrap().as_str().to_string());
        }
    }
    bail!("client_id not found");
}

async fn get_client_id(ctx: &ExtractionContext, force: bool) -> Result<String> {
    if !force {
        if let Ok(Some(cid)) = ctx.cache.get("soundcloud_client_id", "_").await {
            return Ok(cid);
        }
    }
    let cid = extract_client_id(ctx).await?;
    ctx.cache.set("soundcloud_client_id", "_", &cid).await?;
    return Ok(cid);
}

/*
async fn do_post_api_request<Q>(
    ctx: &ExtractionContext,
    resource_name: &str,
    path: &str,
    params: &mut QString,
    payload: &Q,
    force_get_client_id: bool,
) -> Result<Response>
where
    Q: Serialize,
{
    let mut url = Url::parse("https://api-v2.soundcloud.com/")?.join(path)?;
    params.add_pair(("client_id", get_client_id(ctx, force_get_client_id).await?));
    url.set_query(Some(&params.to_string()));
    Ok(ctx
        .send_request(
            resource_name,
            ctx.http
                .post(url)
                .body_json(payload)
                .map_err(|e| anyhow!(e))?,
        )
        .await
        .map_err(|e| anyhow!(e))?)
}
*/

async fn do_get_api_request(
    ctx: &ExtractionContext,
    resource_name: &str,
    path: &str,
    params: &mut QString,
    force_get_client_id: bool,
) -> Result<Response> {
    let mut url = Url::parse("https://api-v2.soundcloud.com/")?.join(path)?;
    params.add_pair(("client_id", get_client_id(ctx, force_get_client_id).await?));
    url.set_query(Some(&params.to_string()));
    Ok(ctx
        .send_request(resource_name, ctx.http.get(url))
        .await
        .map_err(|e| anyhow!(e))?)
}

/*
pub async fn post_api_request<Q, A>(
    ctx: &ExtractionContext,
    resource_name: &str,
    path: &str,
    params: &mut QString,
    payload: &Q,
) -> Result<A>
where
    Q: Serialize,
    A: for<'a> Deserialize<'a>,
{
    let mut res = do_post_api_request(ctx, resource_name, path, params, payload, false).await?;
    let status = res.status();
    if status == 401 || status == 403 {
        // retry with refreshing client_id
        res = do_post_api_request(ctx, resource_name, path, params, payload, true).await?;
    }
    Ok(res.body_json().await.map_err(|e| anyhow!(e))?)
}
*/

pub async fn get_api_request<A>(
    ctx: &ExtractionContext,
    resource_name: &str,
    path: &str,
    params: &mut QString,
) -> Result<A>
where
    A: for<'a> Deserialize<'a>,
{
    let mut res = do_get_api_request(ctx, resource_name, path, params, false).await?;
    let status = res.status();
    if status == 401 || status == 403 {
        // retry with refreshing client_id
        res = do_get_api_request(ctx, resource_name, path, params, true).await?;
    }
    Ok(res.body_json().await.map_err(|e| anyhow!(e))?)
}
