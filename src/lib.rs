#[macro_use]
extern crate smart_default;
#[macro_use]
extern crate lazy_static;

pub mod extractors;

use std::env;

fn build_http() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    // default, probably overriden by extractors
    headers.append(reqwest::header::USER_AGENT, "okhttp/4.9.3".parse().unwrap());
    headers.append(
        reqwest::header::ACCEPT_LANGUAGE,
        "en-US, en-GB;q=0.9, en;q=0.8".parse().unwrap(),
    );
    let mut builder = reqwest::ClientBuilder::new().default_headers(headers);

    if let Ok(proxy) = env::var("http_proxy") {
        builder = builder
            .danger_accept_invalid_certs(true)
            .proxy(reqwest::Proxy::all(proxy).unwrap());
    }

    builder.build().unwrap()
}
