use reytan_extractor_api::{
    AudioDetails, Extraction, FormatBreed, LiveStatus, MediaFormat, MediaFormatURL, MediaMetadata,
    MediaPlayback,
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

impl From<Transcoding> for MediaFormat {
    fn from(t: Transcoding) -> Self {
        MediaFormat {
            id: t.preset,
            breed: FormatBreed::Audio,
            url: match t.format.protocol {
                MediaProtocol::Progressive => {
                    Box::new(MediaFormatURL::HTTP(t.url.parse().unwrap()))
                }
                MediaProtocol::Hls => Box::new(MediaFormatURL::HLS(t.url.parse().unwrap())),
            },
            video_details: None,
            audio_details: Some(AudioDetails {
                ..Default::default()
            }),
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
    pub media: Media,
    pub user: User,
    pub created_at: String,
    pub last_modified: String,
}

impl From<Track> for Extraction {
    fn from(track: Track) -> Self {
        Extraction {
            metadata: Some(MediaMetadata {
                id: track.id.to_string(),
                title: track.title,
                live_status: Some(LiveStatus::NotLive), // no live functionality
            }),
            playback: Some(MediaPlayback {
                formats: track
                    .media
                    .transcodings
                    .into_iter()
                    .map(MediaFormat::from)
                    .collect(),
            }),
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
