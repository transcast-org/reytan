use std::time::Duration;

use reytan_extractor_api::{
    chrono, AudioDetails, Extraction, FormatBreed, HLSDownloadOptions, HTTPDownloadOptions,
    LiveStatus, MediaFormatDetails, MediaFormatEstablished, MediaFormatURL, MediaMetadata, Utc,
};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct User {
    pub id: u64,
    pub username: Option<String>,
    pub permalink: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MediaProtocol {
    Progressive,
    Hls,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TranscodingFormat {
    pub mime_type: String,
    pub protocol: MediaProtocol,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Transcoding {
    pub url: String,
    pub format: TranscodingFormat,
    pub preset: String,
}

impl From<Transcoding> for MediaFormatEstablished {
    fn from(t: Transcoding) -> Self {
        MediaFormatEstablished {
            details: MediaFormatDetails {
                id: t.preset,
                breed: FormatBreed::Audio,
                video_details: None,
                audio_details: Some(AudioDetails {
                    ..Default::default()
                }),
            },
            url: match t.format.protocol {
                MediaProtocol::Progressive => {
                    MediaFormatURL::HTTP(t.url.parse().unwrap(), HTTPDownloadOptions::default())
                }
                MediaProtocol::Hls => {
                    MediaFormatURL::HLS(t.url.parse().unwrap(), HLSDownloadOptions::default())
                }
            },
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Media {
    pub transcodings: Vec<Transcoding>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub duration: u64,
    pub media: Media,
    pub user: User,
    pub playback_count: u64,
    pub created_at: Option<String>,
    pub release_date: Option<String>,
    pub last_modified: Option<String>,
}

impl From<Track> for Extraction {
    fn from(track: Track) -> Self {
        Extraction {
            metadata: MediaMetadata {
                id: track.id.to_string(),
                title: track.title,
                description: Some(track.description),
                duration: Some(Duration::from_millis(track.duration)),
                view_count: Some(track.playback_count),
                live_status: Some(LiveStatus::NotLive), // no live functionality
                created_time: track
                    .created_at
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc3339)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                published_time: track
                    .release_date
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc3339)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                modified_time: track
                    .last_modified
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc3339)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                ..Default::default()
            },
            established_formats: track
                .media
                .transcodings
                .into_iter()
                .map(MediaFormatEstablished::from)
                .collect(),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
/// At a certain amount of tracks, Soundcloud hates us
/// and sends just the most important informations about tracks in a set
/// which apparently are: id, monetization_model, and policy
pub struct TrackStub {
    pub id: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum MaybeTrackInfo {
    Track(Track),
    Stub(TrackStub),
}

impl MaybeTrackInfo {
    pub fn track(&self) -> Option<Track> {
        match self {
            MaybeTrackInfo::Track(track) => Some(track.clone()),
            _ => None,
        }
    }

    pub fn stub(&self) -> Option<TrackStub> {
        match self {
            MaybeTrackInfo::Stub(stub) => Some(stub.clone()),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Set {
    pub id: u64,
    pub track_count: usize,
    pub title: String,
    pub tracks: Vec<MaybeTrackInfo>,
}
