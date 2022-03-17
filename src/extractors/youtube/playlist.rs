use crate::extractors::api::{
    AnyExtraction, ListBreed, ListContinuation, ListExtraction, ListExtractor,
};
use crate::extractors::youtube::common::innertube_request;
use crate::extractors::youtube::types::request::clients::ANDROID;
use crate::extractors::youtube::types::{request, response};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use super::types::response::parts::{PlaylistVideoListRenderer, Renderer};

#[derive(Clone, Copy)]
pub struct YoutubePlaylistLE {}

impl YoutubePlaylistLE {
    async fn yti_browse(
        self,
        http: &Client,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Browse> {
        let json = request::Browse {
            browse_id: format!("VL{}", id),
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
            browse_id: format!("VL{}", id),
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
}

fn get_videos(renderer: Renderer) -> Option<PlaylistVideoListRenderer> {
    match renderer {
        Renderer::SingleColumnBrowseResultsRenderer { tabs }
        | Renderer::TwoColumnBrowseResultsRenderer { tabs } => {
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
            return Some(
                contents
                    .get(0)
                    .unwrap()
                    .playlist_video_list_renderer
                    .clone(),
            )
        }
    }
    return None;
}

#[async_trait]
impl ListExtractor for YoutubePlaylistLE {
    async fn extract_list_initial(
        self,
        http: &reqwest::Client,
        id: &str,
    ) -> Result<ListExtraction> {
        let pvlr = {
            let browse = self.yti_browse(http, id, &ANDROID).await?;
            println!("{:#?}", browse);
            get_videos(browse.contents.unwrap()).unwrap()
        };

        return Ok(ListExtraction {
            id: id.to_string(),
            breed: ListBreed::Playlist,
            is_endless: false,
            entries: Some(Ok(pvlr
                .contents
                .unwrap()
                .into_iter()
                .map(|v| v.playlist_video_renderer)
                .filter(|v| v.is_some())
                .map(|v| AnyExtraction::Recording(v.unwrap().into()))
                .collect())),
            continuation: if let Some(continuation) = pvlr.continuations.unwrap().get(0) {
                Some(continuation.next_continuation_data.continuation.clone())
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
        let browse = self
            .yti_browse_cont(http, id, &ANDROID, continuation.to_string())
            .await?;
        println!("{:#?}", browse);
        let pvlr = browse
            .continuation_contents
            .unwrap()
            .playlist_video_list_continuation;

        return Ok(ListContinuation {
            id: id.to_string(),
            entries: Some(Ok(pvlr
                .contents
                .unwrap()
                .into_iter()
                .map(|v| v.playlist_video_renderer)
                .filter(|v| v.is_some())
                .map(|v| AnyExtraction::Recording(v.unwrap().into()))
                .collect())),
            continuation: if let Some(continuations) = pvlr.continuations {
                if let Some(continuation) = continuations.get(0) {
                    Some(continuation.next_continuation_data.continuation.clone())
                } else {
                    None
                }
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

    use crate::{
        build_http,
        extractors::api::{AnyExtraction, ListBreed, ListExtractor},
    };

    use super::YoutubePlaylistLE;

    #[tokio::test]
    async fn do_extract_youtube_playlist() {
        let pid = "PLpTn8onHfnD2QpCHU-llSG9hbQUwKIVFr";
        let http = build_http();
        let ytp = YoutubePlaylistLE {};
        let initial = ytp.extract_list_initial(&http, pid).await.unwrap();
        println!("{:#?}", initial);
        assert_eq!(initial.id, pid);
        assert_eq!(initial.breed, ListBreed::Playlist);
        assert_eq!(initial.is_endless, false);
        let stream = stream::unfold(initial.continuation.clone(), |state| {
            let local = http.clone();
            async move {
                if let Some(conti_token) = state {
                    let continuation = ytp
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
}
