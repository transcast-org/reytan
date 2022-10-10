use anyhow::Result;
use async_trait::async_trait;
use reytan_context::reqwest::{self, Client};
use reytan_extractor_api::{
    AnyExtraction, Extraction, ListBreed, ListContinuation, ListExtraction, ListExtractor,
    URLMatcher,
};
use url::Url;

use super::common::innertube_request;
use super::common::YOUTUBE_HOSTS_MAIN;
use super::types::request::clients::ANDROID;
use super::types::response::parts::{ActualVideoListRenderer, Renderer};
use super::types::VideoList;
use super::types::{request, response};

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

impl URLMatcher for YoutubeTabLE {
    fn match_extractor(&self, url: &Url) -> bool {
        Some(url)
            .filter(|u| match u.scheme() {
                "http" | "https" => true,
                _ => false,
            })
            .filter(|u| {
                let host = u.host_str().unwrap();
                let first_segment = u.path_segments().unwrap().next().unwrap();
                YOUTUBE_HOSTS_MAIN.contains(&host)
                    && ["playlist", "channel", "c", "user"].contains(&first_segment)
            })
            .is_some()
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
        &self,
        http: &reqwest::Client,
        url: &Url,
    ) -> Result<ListExtraction> {
        // let (browse_id, params) = pseudo_id_to_id_and_params(id.to_string());
        let navigation_resolve = self
            .yti_navigation_resolve(http, url.as_str(), &ANDROID)
            .await
            .unwrap();
        let browse_end = navigation_resolve.endpoint.browse_endpoint.unwrap();
        let vl: VideoList<Extraction> = {
            let browse = self
                .yti_browse(http, &browse_end.browse_id, &ANDROID, browse_end.params)
                .await?;
            println!("{:#?}", browse);
            get_videos(browse.contents.unwrap()).unwrap().into()
        };
        let breed = if browse_end.browse_id.starts_with("VL") {
            ListBreed::Playlist
        } else {
            ListBreed::Channel
        };

        return Ok(ListExtraction {
            id: browse_end.browse_id,
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
            ..Default::default()
        });
    }

    async fn extract_list_continuation(
        &self,
        http: &reqwest::Client,
        browse_id: &str,
        continuation: &str,
    ) -> Result<ListContinuation> {
        let browse = self
            .yti_browse_cont(http, browse_id, &ANDROID, continuation.to_string())
            .await?;
        println!("{:#?}", browse);
        let pvlr: VideoList<Extraction> = browse.continuation_contents.unwrap().into();

        return Ok(ListContinuation {
            id: browse_id.to_string(),
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
    use reytan_context::build_http;
    use reytan_extractor_api::{AnyExtraction, ListBreed, ListExtractor, URLMatcher};
    use url::Url;

    use super::YoutubeTabLE;

    #[tokio::test]
    async fn do_extract_youtube_playlist() {
        let url =
            Url::parse("https://www.youtube.com/playlist?list=PLpTn8onHfnD2QpCHU-llSG9hbQUwKIVFr")
                .unwrap();
        let http = build_http();
        let ytt = YoutubeTabLE {};
        let initial = ytt.extract_list_initial(&http, &url).await.unwrap();
        println!("{:#?}", initial);
        assert_eq!(initial.id, "VLPLpTn8onHfnD2QpCHU-llSG9hbQUwKIVFr");
        assert_eq!(initial.breed, ListBreed::Playlist);
        assert_eq!(initial.is_endless, false);
        let stream = stream::unfold(initial.continuation.clone(), |state| {
            let local = http.clone();
            let init_id = initial.id.clone();
            async move {
                if let Some(conti_token) = state {
                    let continuation = ytt
                        .extract_list_continuation(&local, &init_id, &conti_token)
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
        let url = &Url::parse("https://www.youtube.com/c/Astrophysicsynth/videos").unwrap();
        let mtch = ytt.match_extractor(url);
        assert_eq!(mtch, true);
        let initial = ytt.extract_list_initial(&http, &url).await.unwrap();
        println!("{:#?}", initial);
        assert_eq!(initial.id, "UCWSC_-y9QsDmACXRY3rvtsQ");
        assert_eq!(initial.breed, ListBreed::Channel);
        assert_eq!(initial.is_endless, false);
        let stream = stream::unfold(initial.continuation.clone(), |state| {
            let local = http.clone();
            let init_id = initial.id.clone();
            async move {
                if let Some(conti_token) = state {
                    let continuation = ytt
                        .extract_list_continuation(&local, &init_id, &conti_token)
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
