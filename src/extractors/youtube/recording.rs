use async_trait::async_trait;
use anyhow::{Result, anyhow};
use reqwest::{self, header, Client};
use crate::extractors::api::{MediaFormat, ExtractLevel};

use super::super::api::{RecordingExtractor, Extraction, MediaMetadata, MediaPlayback, Extractable};
use super::types::request::clients::ANDROID_MUSIC;
use super::types::response::parts::StreamingData;
use super::types::{response, request};

pub struct YoutubeRE {}

impl YoutubeRE {
    async fn yti_player(self, http: &Client, id: &str, client: &request::Client<'_>) -> Result<response::Player> {
        let json = request::Player {
            video_id: id.to_string(),
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            ..Default::default()
        };
        println!("{:?}", json);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, "okhttp/4.9.3".parse()?);
        headers.insert("Sec-Fetch-Mode", "navigate".parse()?);
        headers.insert(header::COOKIE, "PREF=hl=en&tz=UTC; CONSENT=YES+cb.20210328-17-p0.en+FX+929".parse()?);
        headers.insert(header::ORIGIN, format!("https://{}", client.host).parse()?);
        if let Some(client_id) = client.client_id {
            headers.insert("X-Youtube-Client-Name", client_id.into());
        }
        headers.insert("X-Youtube-Client-Version", client.context.client_version.parse()?);
        let resp = http
            .post(format!("https://www.youtube.com/youtubei/v1/player?key={}", client.api_key))
            .json(&json)
            .headers(headers)
            .send()
            .await?
            .json::<response::Player>().await?;
        Ok(resp)
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
    async fn extract_recording(self, http: &reqwest::Client, id: &str, wanted: &Extractable) -> Result<Extraction> {
        let player = self.yti_player(http, id, &ANDROID_MUSIC).await?;
        let fmts = if let Some(stream) = player.streaming_data {
            Some(parse_formats(stream))
        } else {
            None
        };
        Ok(Extraction {
            metadata: Some(Ok(MediaMetadata {
                id: player.video_details.video_id,
                title: player.video_details.title,
                ..Default::default()
            })),
            playback: if let Some(formats) = fmts {
                Some(Ok(MediaPlayback {
                    formats,
                    ..Default::default()
                }))
            } else if wanted.playback != ExtractLevel::None {
                Some(Err(anyhow!(
                    "No playback: {}, {}", 
                    player.playability_status.status, 
                    player.playability_status.reason.unwrap_or("(no reason)".to_string(),
                ))))
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
    use crate::extractors::api::{RecordingExtractor, Extractable, FormatBreed, ExtractLevel};

    use super::YoutubeRE;
    use super::super::types::request::clients::{ANDROID_MUSIC, ANDROID_AGEGATE};

    #[tokio::test]
    async fn do_yti_player_protected() {
        let youtube = YoutubeRE {};
        let response = youtube.yti_player(&build_http(), "KushW6zvazM", &ANDROID_MUSIC).await.expect("yti player");
        println!("{:?}", response);
        assert_eq!(response.playability_status.status, "OK", "playability status");
        assert_ne!(response.streaming_data, None, "no streaming data");
        todo!();
    }

    #[tokio::test]
    async fn do_yti_player_agegate() {
        let youtube = YoutubeRE {};
        let response = youtube.yti_player(&build_http(), "o6wtDPVkKqI", &ANDROID_AGEGATE).await.expect("yti player");
        println!("{:?}", response);
        assert_eq!(response.playability_status.status, "OK", "playability status");
        assert_ne!(response.streaming_data, None, "no streaming data");
    }

    #[tokio::test]
    async fn do_extract_protected() {
        let youtube = YoutubeRE {};
        let response = youtube.extract_recording(&build_http(), "KushW6zvazM", &Extractable {
            metadata: ExtractLevel::Extended,
            playback: ExtractLevel::Extended,
            ..Default::default()
        }).await.expect("player response");
        println!("{:?}", response);
        let meta = response.metadata.expect("metadata").expect("metadata");
        assert_eq!(meta.title, "DECO*27 - ゴーストルール feat. 初音ミク");
        let play = response.playback.expect("playback").expect("playback");
        assert!(play.formats.len() > 0);
        let f251 = play.formats.into_iter()
            .find(|f| f.id == "251")
            .expect("format 251");
        assert_eq!(f251.breed, FormatBreed::Audio);
        assert_eq!(f251.video_details, None);
        let audio = f251.audio_details.expect("251 audio details");
        assert_eq!(audio.channels.unwrap(), 2);
    }
}
