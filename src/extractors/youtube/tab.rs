use crate::extractors::api::{
    AnyExtraction, Extraction, ListBreed, ListContinuation, ListExtraction, ListExtractor,
    URLMatch, URLMatcher,
};
use crate::extractors::youtube::common::innertube_request;
use crate::extractors::youtube::types::request::clients::ANDROID;
use crate::extractors::youtube::types::{request, response};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use url::Url;

use super::common::YOUTUBE_HOSTS_MAIN;
use super::types::response::parts::{ActualVideoListRenderer, Renderer};
use super::types::VideoList;

#[derive(Clone, Copy)]
pub struct YoutubeTabLE {}

impl YoutubeTabLE {
    async fn yti_browse(
        self,
        http: &Client,
        id: &str,
        client: &request::Client<'_>,
        params: Option<String>,
    ) -> Result<response::Browse> {
        let json = request::Browse {
            browse_id: id.to_string(),
            params,
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            ..Default::default()
        };
        println!("{:?}", json);
        innertube_request(http, client, "browse", json).await
    }
    async fn yti_browse_cont(
        self,
        http: &Client,
        id: &str,
        client: &request::Client<'_>,
        continuation: String,
    ) -> Result<response::BrowseContinuation> {
        let json = request::Browse {
            browse_id: id.to_string(),
            continuation: Some(continuation),
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            ..Default::default()
        };
        println!("{:?}", json);
        innertube_request(http, client, "browse", json).await
    }
    async fn yti_navigation_resolve(
        self,
        http: &Client,
        url: &str,
        client: &request::Client<'_>,
    ) -> Result<response::NavigationResolve> {
        let json = request::NavigationResolve {
            url: url.to_string(),
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            ..Default::default()
        };
        println!("{:?}", json);
        innertube_request(http, client, "navigation/resolve_url", json).await
    }
}

fn pseudo_id_to_id_and_params(psid: String) -> (String, Option<String>) {
    let s: Vec<&str> = psid.split("#").collect();
    return (
        s[0].to_string(),
        if let Some(p) = s.get(1) {
            Some(p.to_string())
        } else {
            None
        },
    );
}

#[async_trait]
impl URLMatcher for YoutubeTabLE {
    async fn match_extractor(self, url: &Url, http: &reqwest::Client) -> Option<URLMatch> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        if let Some(hostname) = url.host_str() {
            let segments: Vec<&str> = url.path_segments().unwrap_or("".split('/')).collect();
            if YOUTUBE_HOSTS_MAIN.contains(&hostname) {
                let seg0 = segments.get(0).unwrap_or(&"");
                match seg0 {
                    &"playlist" => {
                        if let Some(v) = url.query_pairs().find(|pair| pair.0 == "list") {
                            return Some(URLMatch {
                                id: format!("VL{}", v.1),
                            });
                        }
                    }
                    &"channel" => {
                        if let Some(id) = segments.get(1) {
                            return Some(URLMatch { id: id.to_string() });
                        }
                    }
                    &"c" | &"user" => {
                        if let Some(name) = segments.get(1) {
                            let nru = self
                                .yti_navigation_resolve(
                                    http,
                                    format!(
                                        "{}://{}/{}/{}/{}",
                                        scheme,
                                        hostname,
                                        seg0,
                                        name,
                                        segments.get(2).unwrap_or(&"videos")
                                    )
                                    .as_ref(),
                                    &ANDROID,
                                )
                                .await
                                .expect("innertube navigation resolve");
                            let be = nru
                                .endpoint
                                .browse_endpoint
                                .expect("navigation resolve browse endpoint");
                            return Some(URLMatch {
                                id: format!(
                                    "{}#{}",
                                    be.browse_id,
                                    be.params.unwrap_or("".to_string())
                                ),
                            });
                        }
                    }
                    _ => (),
                }
            }
        }
        return None;
    }
}

fn get_videos(renderer: Renderer) -> Option<ActualVideoListRenderer> {
    match renderer {
        Renderer::SingleColumnBrowseResultsRenderer { tabs }
        | Renderer::TwoColumnBrowseResultsRenderer { tabs }
        | Renderer::ItemSectionRenderer { contents: tabs } => {
            for tab in tabs {
                let renderer = get_videos(tab);
                if renderer.is_some() {
                    return renderer;
                }
            }
        }
        Renderer::TabRenderer { content } => {
            return get_videos(content.unwrap().as_ref().clone());
        }
        Renderer::SectionListRenderer { contents } => {
            if let Some(items) = contents {
                return Some(items.get(0).unwrap().clone());
            }
        }
        Renderer::PlaylistVideoListRendererWrapper {
            playlist_video_list_renderer,
        } => return Some(playlist_video_list_renderer),
    }
    return None;
}

#[async_trait]
impl ListExtractor for YoutubeTabLE {
    async fn extract_list_initial(
        self,
        http: &reqwest::Client,
        id: &str,
    ) -> Result<ListExtraction> {
        let (browse_id, params) = pseudo_id_to_id_and_params(id.to_string());
        let vl: VideoList<Extraction> = {
            let browse = self.yti_browse(http, &browse_id, &ANDROID, params).await?;
            println!("{:#?}", browse);
            get_videos(browse.contents.unwrap()).unwrap().into()
        };
        let breed = if browse_id.starts_with("VL") {
            ListBreed::Playlist
        } else {
            ListBreed::Channel
        };

        return Ok(ListExtraction {
            id: id.to_string(),
            breed,
            is_endless: false,
            entries: Some(Ok(vl
                .videos
                .into_iter()
                .map(|v| AnyExtraction::Recording(v))
                .collect())),
            continuation: if let Some(continuation) = vl.continuations.get(0) {
                Some(continuation.continuation.clone())
            } else {
                None
            },
        });
    }

    async fn extract_list_continuation(
        self,
        http: &reqwest::Client,
        id: &str,
        continuation: &str,
    ) -> Result<ListContinuation> {
        let (browse_id, _) = pseudo_id_to_id_and_params(id.to_string());
        let browse = self
            .yti_browse_cont(http, &browse_id, &ANDROID, continuation.to_string())
            .await?;
        println!("{:#?}", browse);
        let pvlr: VideoList<Extraction> = browse.continuation_contents.unwrap().into();

        return Ok(ListContinuation {
            id: id.to_string(),
            entries: Some(Ok(pvlr
                .videos
                .into_iter()
                .map(|v| AnyExtraction::Recording(v))
                .collect())),
            continuation: if let Some(continuation) = pvlr.continuations.get(0) {
                Some(continuation.continuation.clone())
            } else {
                None
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use futures::prelude::*;
    use futures::stream;
    use url::Url;

    use crate::extractors::api::URLMatcher;
    use crate::{
        build_http,
        extractors::api::{AnyExtraction, ListBreed, ListExtractor},
    };

    use super::YoutubeTabLE;

    #[tokio::test]
    async fn do_extract_youtube_playlist() {
        // https://www.youtube.com/playlist?list=PLpTn8onHfnD2QpCHU-llSG9hbQUwKIVFr
        let pid = "VLPLpTn8onHfnD2QpCHU-llSG9hbQUwKIVFr";
        let http = build_http();
        let ytt = YoutubeTabLE {};
        let initial = ytt.extract_list_initial(&http, pid).await.unwrap();
        println!("{:#?}", initial);
        assert_eq!(initial.id, pid);
        assert_eq!(initial.breed, ListBreed::Playlist);
        assert_eq!(initial.is_endless, false);
        let stream = stream::unfold(initial.continuation.clone(), |state| {
            let local = http.clone();
            async move {
                if let Some(conti_token) = state {
                    let continuation = ytt
                        .extract_list_continuation(&local, pid, &conti_token)
                        .await
                        .unwrap();
                    Some((
                        continuation
                            .entries
                            .expect("continuation entries")
                            .expect("continuation entries"),
                        continuation.continuation,
                    ))
                } else {
                    None
                }
            }
        });
        let extractions: Vec<AnyExtraction> = initial
            .entries
            .expect("initial entries")
            .expect("initial entries")
            .into_iter()
            .chain(
                stream
                    .collect::<Vec<Vec<AnyExtraction>>>()
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<AnyExtraction>>(),
            )
            .collect();
        assert!(extractions.len() >= 74);
    }

    #[tokio::test]
    async fn do_extract_youtube_channel() {
        let http = build_http();
        let ytt = YoutubeTabLE {};
        let id = &ytt
            .match_extractor(
                &Url::parse("https://www.youtube.com/c/DariaZawia%C5%82owofficial").unwrap(),
                &http,
            )
            .await
            .expect("tab id")
            .id;
        let initial = ytt.extract_list_initial(&http, &id).await.unwrap();
        println!("{:#?}", initial);
        assert!(initial.id.starts_with("UCTmABeIeNXBvh1DUG8EDCSw"));
        assert_eq!(initial.breed, ListBreed::Channel);
        assert_eq!(initial.is_endless, false);
        let stream = stream::unfold(initial.continuation.clone(), |state| {
            let local = http.clone();
            async move {
                if let Some(conti_token) = state {
                    let continuation = ytt
                        .extract_list_continuation(&local, &id, &conti_token)
                        .await
                        .unwrap();
                    Some((
                        continuation
                            .entries
                            .expect("continuation entries")
                            .expect("continuation entries"),
                        continuation.continuation,
                    ))
                } else {
                    None
                }
            }
        });
        let extractions: Vec<AnyExtraction> = initial
            .entries
            .expect("initial entries")
            .expect("initial entries")
            .into_iter()
            .chain(
                stream
                    .collect::<Vec<Vec<AnyExtraction>>>()
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<AnyExtraction>>(),
            )
            .collect();
        assert!(extractions.len() >= 50);
    }
}
