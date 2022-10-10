pub use reqwest;
use sys_locale::get_locale;

#[derive(Clone)]
pub struct ExtractionContext {
    pub http: reqwest::Client,
    pub locales: Vec<String>,
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
        }
    }

    pub fn new_with_locale(locales: Vec<String>) -> ExtractionContext {
        ExtractionContext {
            http: build_http(&locales),
            locales,
        }
    }
}

pub fn build_http(locales: &Vec<String>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    // default, probably overriden by extractors
    headers.append(reqwest::header::USER_AGENT, "okhttp/4.9.3".parse().unwrap());
    headers.append(
        reqwest::header::ACCEPT_LANGUAGE,
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
            .join(",")
            .parse()
            .unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}
