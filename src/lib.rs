#[macro_use]
extern crate smart_default;
#[macro_use]
extern crate lazy_static;

pub mod extractors;

use std::env;

fn build_http() -> reqwest::Client {
    let mut builder = reqwest::ClientBuilder::new()
        // default, probably overriden by extractors
        .user_agent("okhttp/4.9.3");

    if let Ok(proxy) = env::var("http_proxy") {
        builder = builder
            .danger_accept_invalid_certs(true)
            .proxy(reqwest::Proxy::all(proxy).unwrap());
    }
    
    builder
        .build()
        .unwrap()
}
