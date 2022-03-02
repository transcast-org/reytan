use async_trait::async_trait;
use serde::Serialize;
use anyhow::Result;

#[async_trait]
pub trait RecordingExtractor {
    async fn extract_recording(self, http: &reqwest::Client, id: &str, wanted: &Extractable) -> Result<Extraction>;
}

/// What should be extracted from the service.
/// This is to limit the amount of requests made, based on what is needed.
/// Extractors may fail just some parts and return the others in some cases, possible situations include:
/// * rate limiting
/// * region locks
/// * age gate
#[derive(Default)]
pub struct Extractable {
    pub metadata: ExtractLevel,
    pub playback: ExtractLevel,
}

#[derive(Serialize, SmartDefault, PartialEq, Debug)]
pub enum ExtractLevel {
    #[default]
    None,
    Basic,
    Extended,
}

#[derive(Default, Debug)]
pub struct Extraction {
    pub metadata: Option<Result<MediaMetadata>>,
    pub playback: Option<Result<MediaPlayback>>,
}

#[derive(Serialize, Default, PartialEq, Debug)]
pub struct MediaMetadata {
    pub id: String,
    pub title: String,
}

#[derive(Serialize, Default, PartialEq, Debug)]
pub struct MediaPlayback {
    pub formats: Vec<MediaFormat>,
}

#[derive(Serialize, SmartDefault, PartialEq, Debug)]
pub struct MediaFormat {
    pub id: String,
    pub breed: FormatBreed,
    pub url: String,
    pub video_details: Option<VideoDetails>,
    pub audio_details: Option<AudioDetails>,
}

/// Format type
#[derive(Serialize, SmartDefault, PartialEq, Debug)]
pub enum FormatBreed {
    #[default]
    AudioVideo,
    Video,
    Audio,
}

#[derive(Serialize, SmartDefault, PartialEq, Debug)]
pub struct VideoDetails {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Serialize, SmartDefault, PartialEq, Debug)]
pub struct AudioDetails {
    pub channels: Option<u8>,
}
