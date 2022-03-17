use crate::extractors::api::{
    Extractable, Extraction, MediaFormat, MediaMetadata, MediaPlayback, RecordingExtractor,
    URLMatch, URLMatcher,
};
use crate::extractors::youtube::common::{
    innertube_request, YOUTUBE_HOSTS_MAIN, YOUTUBE_HOSTS_SHORT,
};
use crate::extractors::youtube::types::{
    request::{self, clients::ANDROID_MUSIC},
    response::{self, parts::StreamingData},
};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{self, Client};
use url::Url;

pub struct YoutubeRE {}

impl YoutubeRE {
    async fn yti_player(
        self,
        http: &Client,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        let json = request::Player {
            video_id: id.to_string(),
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            ..Default::default()
        };
        println!("{:?}", json);
        innertube_request(http, client, "player", json).await
    }
}

#[async_trait]
impl URLMatcher for YoutubeRE {
    async fn match_extractor(self, url: &Url) -> Option<URLMatch> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        if let Some(hostname) = url.host_str() {
            let segments: Vec<&str> = url.path_segments().unwrap_or("".split('/')).collect();
            if YOUTUBE_HOSTS_MAIN.contains(&hostname) {
                match segments.get(0).unwrap_or(&"") {
                    &"watch" => {
                        if let Some(v) = url.query_pairs().find(|pair| pair.0 == "v") {
                            return Some(URLMatch {
                                id: v.1.to_string(),
                            });
                        }
                    }
                    &"video" | &"shorts" => {
                        if let Some(id) = segments.get(1) {
                            return Some(URLMatch { id: id.to_string() });
                        }
                    }
                    _ => (),
                }
            }
            if YOUTUBE_HOSTS_SHORT.contains(&hostname) {
                if let Some(id) = segments.get(0) {
                    return Some(URLMatch { id: id.to_string() });
                }
            }
        }
        return None;
    }
}

fn parse_formats(strm: StreamingData) -> Vec<MediaFormat> {
    let mut fmts: Vec<MediaFormat> = vec![];
    if let Some(formats) = strm.formats {
        for fmt in formats {
            fmts.push(fmt.into());
        }
    }
    if let Some(formats) = strm.adaptive_formats {
        for fmt in formats {
            fmts.push(fmt.into());
        }
    }
    fmts
}

#[async_trait]
impl RecordingExtractor for YoutubeRE {
    async fn extract_recording(
        self,
        http: &reqwest::Client,
        id: &str,
        _wanted: &Extractable,
    ) -> Result<Extraction> {
        let player = self.yti_player(http, id, &ANDROID_MUSIC).await?;
        let fmts = if let Some(stream) = player.streaming_data {
            Some(parse_formats(stream))
        } else {
            None
        };
        Ok(Extraction {
            metadata: Some(MediaMetadata {
                id: player.video_details.video_id,
                title: player.video_details.title,
                ..Default::default()
            }),
            playback: if let Some(formats) = fmts {
                Some(MediaPlayback {
                    formats,
                    ..Default::default()
                })
            } else {
                None
            },
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::build_http;
    use crate::extractors::api::{
        ExtractLevel, Extractable, FormatBreed, RecordingExtractor, URLMatcher,
    };
    use url::Url;

    use super::super::types::request::clients::{ANDROID_AGEGATE, ANDROID_MUSIC};
    use super::YoutubeRE;

    #[tokio::test]
    async fn do_yti_player_protected() {
        let youtube = YoutubeRE {};
        let response = youtube
            .yti_player(&build_http(), "KushW6zvazM", &ANDROID_MUSIC)
            .await
            .expect("yti player");
        println!("{:?}", response);
        assert_eq!(
            response.playability_status.status, "OK",
            "playability status"
        );
        assert_ne!(response.streaming_data, None, "no streaming data");
    }

    #[tokio::test]
    async fn do_yti_player_agegate() {
        let youtube = YoutubeRE {};
        let response = youtube
            .yti_player(&build_http(), "o6wtDPVkKqI", &ANDROID_AGEGATE)
            .await
            .expect("yti player");
        println!("{:?}", response);
        assert_eq!(
            response.playability_status.status, "OK",
            "playability status"
        );
        assert_ne!(response.streaming_data, None, "no streaming data");
    }

    #[tokio::test]
    async fn do_extract_protected() {
        let youtube = YoutubeRE {};
        let response = youtube
            .extract_recording(
                &build_http(),
                "KushW6zvazM",
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                    ..Default::default()
                },
            )
            .await
            .expect("player response");
        println!("{:?}", response);
        let meta = response.metadata.expect("metadata");
        assert_eq!(meta.title, "DECO*27 - ゴーストルール feat. 初音ミク");
        let play = response.playback.expect("playback");
        assert!(play.formats.len() > 0);
        let f251 = play
            .formats
            .into_iter()
            .find(|f| f.id == "251")
            .expect("format 251");
        assert_eq!(f251.breed, FormatBreed::Audio);
        assert_eq!(f251.video_details, None);
        let audio = f251.audio_details.expect("251 audio details");
        assert_eq!(audio.channels.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_url_match_watch() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap())
            .await
            .unwrap();
        assert_eq!(url_match.id, "dQw4w9WgXcQ");
    }

    #[tokio::test]
    async fn test_url_match_video() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/video/dQw4w9WgXcQ").unwrap())
            .await
            .unwrap();
        assert_eq!(url_match.id, "dQw4w9WgXcQ");
    }

    #[tokio::test]
    async fn test_url_match_shorts() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/shorts/dQw4w9WgXcQ").unwrap())
            .await
            .unwrap();
        assert_eq!(url_match.id, "dQw4w9WgXcQ");
    }

    #[tokio::test]
    async fn test_url_match_shortener() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap())
            .await
            .unwrap();
        assert_eq!(url_match.id, "dQw4w9WgXcQ");
    }
}
