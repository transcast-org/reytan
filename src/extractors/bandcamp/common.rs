use url::Url;

pub fn _path_is(url: &&Url, entity_name: &str) -> bool {
    url.path_segments().map(|s| s.clone().next()).flatten() == Some(entity_name)
}

pub fn _is_bandcamp(url: &&Url) -> bool {
    if let Some(h) = url.host_str() {
        h.ends_with(".bandcamp.com")
    } else {
        false
    }
}
