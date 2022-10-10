#[macro_use]
extern crate smart_default;

use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use url::Url;

pub trait URLMatcher {
    fn match_extractor(&self, url: &Url) -> bool;
}

#[async_trait]
pub trait RecordingExtractor {
    async fn extract_recording(
        &self,
        http: &reqwest::Client,
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

#[derive(Default, Clone, Debug)]
pub struct Extraction {
    pub metadata: Option<MediaMetadata>,
    pub playback: Option<MediaPlayback>,
}

#[derive(Serialize, Default, PartialEq, Clone, Debug)]
pub struct MediaMetadata {
    pub id: String,
    pub title: String,
    pub live_status: Option<LiveStatus>,
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

#[derive(Serialize, Default, PartialEq, Clone, Debug)]
pub struct MediaPlayback {
    pub formats: Vec<MediaFormat>,
}

#[derive(Serialize, SmartDefault, PartialEq, Clone, Debug)]
pub struct MediaFormat {
    pub id: String,
    pub breed: FormatBreed,
    pub url: String,
    pub video_details: Option<VideoDetails>,
    pub audio_details: Option<AudioDetails>,
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

/// Used as a result of the list extraction.
/// Some extractors may return another, nested lists, examples:
///     * volumes of an album
///     * channel's playlists
///     * artist's albums (and the songs)
#[derive(Debug)]
pub enum AnyExtraction {
    Recording(Extraction),
    List(ListExtraction),
}

/// What the list extractor spits out at you.
#[derive(Default, Debug)]
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
#[derive(Default, Debug)]
pub struct ListContinuation {
    pub id: String,
    pub entries: Option<Result<Vec<AnyExtraction>>>,
    /// Gets returned if there are more items (like a next page).
    /// Pass it as `continuation` to ListExtractor.extract_list, in order to fetch more items.
    pub continuation: Option<String>,
}

#[async_trait]
pub trait ListExtractor {
    /// Extracts something that is a list from the service.
    ///
    /// `continuation` is to be used if you're fetching the next portion (page) of the list, as returned in ListExtraction.continuation.
    async fn extract_list_initial(
        &self,
        http: &reqwest::Client,
        url: &Url,
    ) -> Result<ListExtraction>;

    /// `id` and `continuation` parameters provided by the extract_list_inital method
    async fn extract_list_continuation(
        &self,
        http: &reqwest::Client,
        id: &str,
        continuation: &str,
    ) -> Result<ListContinuation>;
}
