#[macro_use]
extern crate smart_default;

mod context;

pub use context::{build_http, ExtractionContext};

pub mod cache;

pub use anyhow;
pub use async_trait::async_trait;
pub use chrono::{self, DateTime, Utc};
pub use http_types::headers;
pub use surf;
pub use url;

use anyhow::Result;
use serde::Serialize;
use std::time::Duration;
use url::Url;

pub trait NewExtractor {
    fn new() -> Self;
}

pub trait URLMatcher {
    fn match_extractor(&self, url: &Url) -> bool;
}

#[async_trait]
pub trait RecordingExtractor: URLMatcher + Sync + Send {
    async fn extract_recording(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<Extraction>;
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

#[derive(Default)]
pub struct Extraction {
    pub metadata: Option<MediaMetadata>,
    pub established_formats: Option<Vec<MediaFormatEstablished>>,
    pub subtitles: Option<Vec<Subtitle>>,
}

#[derive(Serialize, Default, PartialEq, Clone, Debug)]
pub struct MediaMetadata {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub duration: Option<Duration>,
    pub view_count: Option<u64>,
    pub live_status: Option<LiveStatus>,
    pub age_limit: Option<u8>,
    pub created_time: Option<DateTime<Utc>>,
    pub published_time: Option<DateTime<Utc>>,
    pub modified_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub enum LiveStatus {
    /// Never meant as a live stream
    NotLive,
    /// Is a live stream that is currently ongoing
    IsLive,
    /// A recording of a live stream
    WasLive,
}

pub struct MediaFormatEstablished {
    pub details: MediaFormatDetails,
    pub url: MediaFormatURL,
}

pub struct MediaFormatDetails {
    pub id: String,
    pub breed: FormatBreed,
    pub video_details: Option<VideoDetails>,
    pub audio_details: Option<AudioDetails>,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MediaFormatURL {
    HTTP(Url),
    HLS(Url),
}

#[async_trait]
pub trait MediaFormatPointer {
    async fn get(&self) -> Result<MediaFormatURL>;
}

/// Format type
#[derive(Serialize, SmartDefault, PartialEq, Clone, Debug)]
pub enum FormatBreed {
    #[default]
    AudioVideo,
    Video,
    Audio,
}

#[derive(Serialize, SmartDefault, PartialEq, Clone, Debug)]
pub struct VideoDetails {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Serialize, SmartDefault, PartialEq, Clone, Debug)]
pub struct AudioDetails {
    pub channels: Option<u8>,
}

#[derive(SmartDefault, PartialEq, Clone, Debug)]
pub enum ListBreed {
    /// User-defined set of music (incl. liked videos)
    #[default]
    Playlist,
    /// Anything uploaded by a channel/user account
    Channel,
    /// An album, as defined by publisher
    Album,
    /// Machine-defined set of music, probably (virtually) endless
    /// (see: YouTube Mixes based on a song, Spotify/Tidal artist radio)
    Mix,
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub enum SubtitleExt {
    /// WebVTT - https://www.w3.org/TR/webvtt1/
    VTT,
    /// SubRip Text - https://www.matroska.org/technical/subtitles.html#srt-subtitles
    SRT,
    /// Timed Text Markup Language - https://www.w3.org/TR/ttml1/
    TTML,
    /// Advanced SubStation Alpha (if extended from SSA) - https://en.wikipedia.org/wiki/SubStation_Alpha#Advanced_SubStation_Alpha
    ASS,
    /// SubStation Alpha (if not extended to ASS) - https://en.wikipedia.org/wiki/SubStation_Alpha
    SSA,
    /// service-specific format that is not following any industry standards, such as YouTube's "srv3" or "json3"
    NonStandard(String),
}

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct Subtitle {
    pub lang: String,
    pub is_original_lang: Option<bool>,
    pub is_machine_generated: Option<bool>,
    pub is_machine_translated: Option<bool>,
    pub ext: SubtitleExt,
    pub url: Url,
}

/// Used as a result of the list extraction.
/// Some extractors may return another, nested lists, examples:
///     * volumes of an album
///     * channel's playlists
///     * artist's albums (and the songs)
pub enum AnyExtraction {
    Recording(Extraction),
    List(ListExtraction),
}

/// What the list extractor spits out at you.
#[derive(Default)]
pub struct ListExtraction {
    pub id: String,
    pub breed: ListBreed,
    pub title: String,
    pub is_endless: bool,
    pub entries: Option<Result<Vec<AnyExtraction>>>,
    /// Gets returned if there are more items (like a next page).
    /// Pass it as `continuation` to ListExtractor.extract_list, in order to fetch more items.
    pub continuation: Option<String>,
}

/// What the list extractor spits out at you (again, if you want more)
#[derive(Default)]
pub struct ListContinuation {
    pub id: String,
    pub entries: Option<Result<Vec<AnyExtraction>>>,
    /// Gets returned if there are more items (like a next page).
    /// Pass it as `continuation` to ListExtractor.extract_list, in order to fetch more items.
    pub continuation: Option<String>,
}

#[async_trait]
pub trait ListExtractor: URLMatcher + Sync + Send {
    /// Extracts something that is a list from the service.
    ///
    /// `continuation` is to be used if you're fetching the next portion (page) of the list, as returned in ListExtraction.continuation.
    async fn extract_list_initial(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
    ) -> Result<ListExtraction>;

    /// `id` and `continuation` parameters provided by the extract_list_inital method
    async fn extract_list_continuation(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        continuation: &str,
    ) -> Result<ListContinuation>;
}

pub enum AnyExtractor {
    Recording(Box<dyn RecordingExtractor>),
    List(Box<dyn ListExtractor>),
}

impl AnyExtractor {
    pub async fn extract_info(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<AnyExtraction> {
        match self {
            AnyExtractor::Recording(re) => re
                .extract_recording(ctx, url, wanted)
                .await
                .map(AnyExtraction::Recording),
            AnyExtractor::List(le) => le
                .extract_list_initial(ctx, url)
                .await
                .map(AnyExtraction::List),
        }
    }

    pub fn match_extractor(&self, url: &Url) -> bool {
        match self {
            AnyExtractor::Recording(re) => re.match_extractor(url),
            AnyExtractor::List(le) => le.match_extractor(url),
        }
    }
}
