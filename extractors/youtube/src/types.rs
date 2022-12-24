use self::response::parts::Continuation;

pub mod response {
    pub mod parts {
        use once_cell::sync::Lazy;
        use reytan_extractor_api::{
            self as api, url::Url, Extraction, FormatBreed, MediaFormatDetails,
            MediaFormatEstablished, MediaFormatURL, MediaMetadata, SubtitleExt,
        };
        use serde::Deserialize;
        use serde_aux::prelude::*;

        use super::super::VideoList;

        #[derive(SmartDefault, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Format {
            /// innertube format id
            pub itag: u16,
            /// url to download - not present in web
            pub url: Option<String>,
            /// the shit that is present in web instead of url
            pub signature_cipher: Option<String>,
            /// bitrate
            pub bitrate: Option<u64>,
            /// average bitrate
            pub average_bitrate: Option<u64>,
            /// mime type, contains the type (audio/video), container and used codecs
            pub mime_type: String,
            /// file size in bytes
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub content_length: Option<u64>,
            /// video width
            pub width: Option<u32>,
            /// video height
            pub height: Option<u32>,
            /// frames per second
            pub fps: Option<u16>,
            /// audio sample rate
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub audio_sample_rate: Option<u64>,
            /// amount of audio channels (mono, stereo)
            pub audio_channels: Option<u8>,
        }

        impl From<Format> for MediaFormatEstablished {
            fn from(fmt: Format) -> MediaFormatEstablished {
                let is_hls = fmt.mime_type.starts_with("application/x-mpegURL");
                let breed = if fmt.mime_type.starts_with("audio/") {
                    FormatBreed::Audio
                // multiple codecs - "video/3gpp; codecs=\"mp4v.20.3, mp4a.40.2\""
                } else if fmt.mime_type.contains(", ") {
                    FormatBreed::AudioVideo
                } else if is_hls {
                    // not clearly indicated. we can only assume by heuristics (as it is)
                    // or by the indicated codecs. note: A/V covered by the previous check
                    if fmt.audio_channels.is_some() {
                        FormatBreed::Audio
                    } else {
                        FormatBreed::Video
                    }
                } else {
                    FormatBreed::Video
                };
                MediaFormatEstablished {
                    details: MediaFormatDetails {
                        id: fmt.itag.to_string(),
                        video_details: if breed == FormatBreed::Video
                            || breed == FormatBreed::AudioVideo
                        {
                            Some(api::VideoDetails {
                                width: fmt.width,
                                height: fmt.height,
                                ..Default::default()
                            })
                        } else {
                            None
                        },
                        audio_details: if breed == FormatBreed::Audio
                            || breed == FormatBreed::AudioVideo
                        {
                            Some(api::AudioDetails {
                                channels: fmt.audio_channels,
                                ..Default::default()
                            })
                        } else {
                            None
                        },
                        breed,
                    },
                    url: if is_hls {
                        MediaFormatURL::HLS(fmt.url.unwrap().parse().unwrap())
                    } else {
                        MediaFormatURL::HTTP(fmt.url.unwrap().parse().unwrap())
                    },
                }
            }
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct StreamingData {
            // not present in ios responses
            pub formats: Option<Vec<Format>>,
            // not present in ios_creator responses
            pub adaptive_formats: Option<Vec<Format>>,
            // present in ios responses on livestreams
            pub hls_formats: Option<Vec<Format>>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Default, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlayabilityStatus {
            pub status: String,
            pub reason: Option<String>,
            pub reason_title: Option<String>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Default, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct VideoDetails {
            pub video_id: String,
            pub title: String,
            pub author: String,
            pub channel_id: String,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub length_seconds: Option<u64>,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub view_count: Option<u64>,
            pub short_description: Option<String>,
            pub keywords: Option<Vec<String>>,
            #[serde(default)]
            pub is_live: bool,
            #[serde(default)]
            pub is_live_content: bool,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct MicroformatsWrapper {
            pub player_microformat_renderer: Option<Microformats>,
            pub microformat_data_renderer: Option<MicroformatsMusic>,
        }

        /// Microformats for web and web_* EXCEPT web_music
        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Microformats {
            /// ISO 3166-1 alpha-2, uppercase
            pub available_countries: Option<Vec<String>>,
            pub category: Option<String>,
            pub description: Option<RunsWrapper>,
            pub title: Option<RunsWrapper>,
            pub is_family_safe: Option<bool>,
            pub is_unlisted: Option<bool>,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub length_seconds: Option<u64>,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub view_count: Option<u64>,
            /// ISO 8601 date
            pub publish_date: Option<String>,
            /// ISO 8601 date
            pub upload_date: Option<String>,
        }

        /// Very special microformats for very special web_music
        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct MicroformatsMusic {
            /// ISO 3166-1 alpha-2, uppercase
            pub available_countries: Option<Vec<String>>,
            pub category: Option<String>,
            pub description: Option<String>,
            pub title: Option<String>,
            pub family_safe: Option<bool>,
            pub unlisted: Option<bool>,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub view_count: Option<u64>,
            /// ISO 8601 date
            pub publish_date: Option<String>,
            /// ISO 8601 date
            pub upload_date: Option<String>,
            pub video_details: Option<MicroformatsMusicVideoDetails>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct MicroformatsMusicVideoDetails {
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub duration_seconds: Option<u64>,
            pub external_video_id: Option<String>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct CaptionsWrapper {
            pub player_captions_tracklist_renderer: PlayerCaptionsTracklistRenderer,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlayerCaptionsTracklistRenderer {
            pub caption_tracks: Option<Vec<CaptionTrack>>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct CaptionTrack {
            pub base_url: String,
            pub language_code: String,
            pub name: RunsWrapper,
            // Some("asr") - automatic captions
            // None - normal captions
            pub kind: Option<String>,
        }

        static SUBTITLE_EXTS: Lazy<Vec<(&'static str, SubtitleExt)>> = Lazy::new(|| {
            vec![
                ("vtt", SubtitleExt::VTT),
                ("ttml", SubtitleExt::TTML),
                ("srv3", SubtitleExt::NonStandard(String::from("srv3"))),
                ("srv2", SubtitleExt::NonStandard(String::from("srv2"))),
                ("srv1", SubtitleExt::NonStandard(String::from("srv1"))),
                ("json3", SubtitleExt::NonStandard(String::from("json3"))),
            ]
        });

        impl From<PlayerCaptionsTracklistRenderer> for Option<Vec<api::Subtitle>> {
            fn from(r: PlayerCaptionsTracklistRenderer) -> Self {
                if let Some(caption_tracks) = r.caption_tracks {
                    Some(
                        caption_tracks
                            .into_iter()
                            .flat_map(|t| {
                                let base_url = Url::parse(&t.base_url).unwrap();
                                let base_query = qstring::QString::new(
                                    base_url.query_pairs().filter(|(k, _)| k != "fmt").collect(),
                                );
                                let mut result = vec![];
                                for (e, se) in SUBTITLE_EXTS.iter() {
                                    let mut url = base_url.clone();
                                    let mut query = base_query.clone();
                                    query.add_pair(("fmt", *e));
                                    url.set_query(Some(&query.to_string()));
                                    result.push(api::Subtitle {
                                        lang: t.language_code.clone(),
                                        is_original_lang: None,
                                        is_machine_generated: Some(
                                            t.kind == Some("asr".to_string()),
                                        ),
                                        is_machine_translated: Some(false),
                                        ext: se.clone(),
                                        url,
                                    });
                                }
                                result
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            }
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct RunsWrapper {
            pub simple_text: Option<String>,
            pub runs: Option<Vec<RunsInner>>,
        }

        impl From<RunsWrapper> for String {
            fn from(runsw: RunsWrapper) -> Self {
                if let Some(txt) = runsw.simple_text {
                    return txt;
                }
                let mut s = String::new();
                if let Some(runs) = runsw.runs {
                    for p in runs {
                        s.push_str(&p.text);
                    }
                }
                return s.to_string();
            }
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct RunsInner {
            pub text: String,
            pub navigation_endpoint: Option<NavigationEndpoint>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct NavigationEndpoint {
            pub browse_endpoint: Option<BrowseEndpoint>,
        }

        #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct BrowseEndpoint {
            pub browse_id: String,
            pub params: Option<String>,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub enum Renderer {
            /// { "twoColumnBrowseResultsRenderer": { "tabs": [ "tabRenderer": { .. } ] } }
            TwoColumnBrowseResultsRenderer {
                tabs: Vec<Renderer>,
            },
            SingleColumnBrowseResultsRenderer {
                tabs: Vec<Renderer>,
            },
            TabRenderer {
                content: Option<Box<Renderer>>,
            },
            SectionListRenderer {
                contents: Option<Vec<ActualVideoListRenderer>>,
            },
            ItemSectionRenderer {
                contents: Vec<Renderer>,
            },
            PlaylistVideoListRendererWrapper {
                playlist_video_list_renderer: ActualVideoListRenderer,
            },
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaylistVideoListRendererWrapper {
            pub playlist_video_list_renderer: PlaylistVideoListRenderer,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub enum ActualVideoListRenderer {
            PlaylistVideoListRenderer {
                contents: Option<Vec<PlaylistVideoRendererWrapper>>,
                continuations: Option<Vec<ContinuationWrapper>>,
            },
            PlaylistVideoListContinuation {
                contents: Option<Vec<PlaylistVideoRendererWrapper>>,
                continuations: Option<Vec<ContinuationWrapper>>,
            },
            ItemSectionRenderer {
                contents: Option<Vec<ElementRendererWrapper>>,
                continuations: Option<Vec<ContinuationWrapper>>,
            },
            ItemSectionContinuation {
                contents: Option<Vec<ElementRendererWrapper>>,
                continuations: Option<Vec<ContinuationWrapper>>,
            },
        }

        impl From<ActualVideoListRenderer> for VideoList<Extraction> {
            fn from(avlr: ActualVideoListRenderer) -> Self {
                match avlr {
                    ActualVideoListRenderer::PlaylistVideoListRenderer {
                        contents,
                        continuations,
                    }
                    | ActualVideoListRenderer::PlaylistVideoListContinuation {
                        contents,
                        continuations,
                    } => VideoList::<Extraction> {
                        videos: contents
                            .unwrap_or_default()
                            .into_iter()
                            .map(|pvrw| pvrw.playlist_video_renderer)
                            .filter(|pvr| pvr.is_some())
                            .map(|pvr| pvr.unwrap())
                            .map(|pvr| pvr.into())
                            .collect(),
                        continuations: continuations
                            .unwrap_or_default()
                            .into_iter()
                            .map(|cw| cw.next_continuation_data)
                            .collect(),
                    },
                    ActualVideoListRenderer::ItemSectionRenderer {
                        contents,
                        continuations,
                    }
                    | ActualVideoListRenderer::ItemSectionContinuation {
                        contents,
                        continuations,
                    } => VideoList::<Extraction> {
                        videos: contents
                            .unwrap_or_default()
                            .into_iter()
                            .map(|erw| {
                                erw.element_renderer
                                    .new_element
                                    .typee
                                    .component_type
                                    .model
                                    .compact_video_model
                                    .into()
                            })
                            .collect(),
                        continuations: continuations
                            .unwrap_or_default()
                            .into_iter()
                            .map(|cw| cw.next_continuation_data)
                            .collect(),
                    },
                }
            }
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaylistVideoListContinuationWrapper {
            pub playlist_video_list_continuation: PlaylistVideoListRenderer,
        }

        impl From<PlaylistVideoListContinuationWrapper> for VideoList<PlaylistVideoRenderer> {
            fn from(pvlcw: PlaylistVideoListContinuationWrapper) -> Self {
                pvlcw.playlist_video_list_continuation.into()
            }
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaylistVideoListRenderer {
            pub contents: Option<Vec<PlaylistVideoRendererWrapper>>,
            pub continuations: Option<Vec<ContinuationWrapper>>,
        }

        impl From<PlaylistVideoListRenderer> for VideoList<PlaylistVideoRenderer> {
            fn from(plvr: PlaylistVideoListRenderer) -> Self {
                VideoList::<PlaylistVideoRenderer> {
                    videos: plvr
                        .contents
                        .unwrap()
                        .into_iter()
                        .map(|vrw| vrw.playlist_video_renderer)
                        .filter(|vr| vr.is_some())
                        .map(|vr| vr.unwrap())
                        .collect(),
                    continuations: plvr
                        .continuations
                        .unwrap_or_default()
                        .into_iter()
                        .map(|cw| cw.next_continuation_data)
                        .collect(),
                }
            }
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ContinuationWrapper {
            pub next_continuation_data: Continuation,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Continuation {
            pub continuation: String,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaylistVideoRendererWrapper {
            pub playlist_video_renderer: Option<PlaylistVideoRenderer>,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaylistVideoRenderer {
            // this piece of shit stores the video id in ANDROID (I fucking hate innertube)
            pub binding: VideoClientBindingDataWrapper,
            pub title: RunsWrapper,
            pub index: Option<RunsWrapper>,
            pub short_byline_text: Option<RunsWrapper>,
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub length_seconds: Option<u64>,
            pub is_playable: Option<bool>,
        }

        impl From<PlaylistVideoRenderer> for Extraction {
            fn from(vr: PlaylistVideoRenderer) -> Self {
                Self {
                    metadata: MediaMetadata {
                        id: vr.binding.video_client_binding_data.video_id,
                        title: vr.title.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct VideoClientBindingDataWrapper {
            pub video_client_binding_data: VideoClientBindingData,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct VideoClientBindingData {
            pub video_id: String,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ElementRendererWrapper {
            pub element_renderer: ElementRenderer,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ElementRenderer {
            pub new_element: NewElement,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct NewElement {
            #[serde(rename = "type")]
            /// "type" from JSON, renamed due to "type" being a reserved keyword
            pub typee: NewElementType,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct NewElementType {
            pub component_type: ComponentType,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ComponentType {
            pub model: ComponentModel,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ComponentModel {
            pub compact_video_model: CompactVideoModel,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct CompactVideoModel {
            pub compact_video_data: CompactVideoData,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct CompactVideoData {
            pub video_data: VideoData,
            pub on_tap: OnTap,
        }

        impl From<CompactVideoModel> for Extraction {
            fn from(cvm: CompactVideoModel) -> Self {
                Extraction {
                    metadata: MediaMetadata {
                        id: cvm
                            .compact_video_data
                            .on_tap
                            .innertube_command
                            .watch_endpoint
                            .video_id,
                        title: cvm.compact_video_data.video_data.metadata.title,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct VideoData {
            pub metadata: VideoMetadata,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct VideoMetadata {
            pub title: String,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct OnTap {
            pub innertube_command: InnertubeCommand,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct InnertubeCommand {
            pub watch_endpoint: WatchEndpoint,
        }

        #[derive(Deserialize, PartialEq, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct WatchEndpoint {
            pub video_id: String,
        }
    }

    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Eq, Hash, Clone, Default, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/player`
    pub struct Player {
        pub captions: Option<parts::CaptionsWrapper>,
        pub streaming_data: Option<parts::StreamingData>,
        pub playability_status: parts::PlayabilityStatus,
        pub video_details: parts::VideoDetails,
        pub microformat: Option<parts::MicroformatsWrapper>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/browse`
    pub struct Browse {
        pub contents: Option<parts::Renderer>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/browse`
    pub struct BrowseContinuation {
        pub continuation_contents: Option<parts::ActualVideoListRenderer>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/navigation/resolve_url`
    pub struct NavigationResolve {
        pub endpoint: parts::NavigationEndpoint,
    }
}

pub mod request {
    pub mod parts {
        use serde::Serialize;

        #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ThirdParty<'a> {
            pub embed_url: &'a str,
        }

        #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ContextClient<'a> {
            pub client_name: &'a str,
            pub client_version: &'a str,
            pub client_screen: Option<&'a str>,
            pub device_model: Option<&'a str>,
            pub hl: Option<&'a str>,
            #[default = "UTC"]
            pub time_zone: &'a str,
            #[default = 0]
            pub utc_offset_minutes: u8,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Context<'a> {
            pub client: ContextClient<'a>,
            pub third_party: Option<ThirdParty<'a>>,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ContentPlaybackContext {
            #[default = "HTML5_PREF_WANTS"]
            pub html5_preference: String,
            pub signature_timestamp: Option<u32>,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaybackContext {
            pub content_playback_context: ContentPlaybackContext,
        }
    }

    use serde::Serialize;
    use smart_default::SmartDefault;

    #[derive(Serialize, Clone, Copy, Debug)]
    pub struct ImpersonationTarget<'a> {
        /// curl-impersonate target name
        pub target: &'a str,
        /// user-agent header, if not curl-impersonate default for the target
        pub user_agent: Option<&'a str>,
    }

    #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Client<'a> {
        pub name: &'a str,
        pub client_id: Option<u16>,
        #[default = "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8"]
        pub api_key: &'a str,
        pub context: parts::ContextClient<'a>,
        pub third_party: Option<parts::ThirdParty<'a>>,
        #[default = "www.youtube.com"]
        pub host: &'a str,
        pub js_needed: bool,
        /// used if impersonate_chrome/impersonate_ff feature is not used
        pub user_agent: Option<&'a str>,
        /// TODO: used if is Some and impersonate_chrome feature is turned on
        pub chrome_target: Option<ImpersonationTarget<'a>>,
        /// TODO: used if is Some impersonate_ff feature is turned on
        pub ff_target: Option<ImpersonationTarget<'a>>,
    }

    /// INNERTUBE_CLIENTS from yt-dlp: https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube.py
    pub mod clients {
        use super::{
            parts::{ContextClient, ThirdParty},
            Client, ImpersonationTarget,
        };
        pub static ANDROID_MUSIC: Client = Client {
            name: "android_music",
            client_id: Some(21),
            api_key: "AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI",
            context: ContextClient {
                client_name: "ANDROID_MUSIC",
                client_version: "4.57",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static ANDROID: Client = Client {
            name: "android",
            client_id: Some(3),
            api_key: "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w",
            context: ContextClient {
                client_name: "ANDROID",
                client_version: "16.49",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static ANDROID_EMBEDDED: Client = Client {
            name: "android_embedded",
            client_id: Some(55),
            api_key: "AIzaSyCjc_pVEDi4qsv5MtC2dMXzpIaDoRFLsxw",
            context: ContextClient {
                client_name: "ANDROID_EMBEDDED_PLAYER",
                client_version: "16.49",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static ANDROID_CREATOR: Client = Client {
            name: "android_creator",
            client_id: Some(14),
            api_key: "AIzaSyD_qjV8zaaUMehtLkrKFgVeSX_Iqbtyws8",
            context: ContextClient {
                client_name: "ANDROID_CREATOR",
                client_version: "21.47",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static IOS: Client = Client {
            name: "ios",
            client_id: Some(5),
            api_key: "AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc",
            context: ContextClient {
                client_name: "IOS",
                client_version: "16.46",
                client_screen: None,
                device_model: Some("iPhone14,3"),
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static IOS_EMBEDDED: Client = Client {
            name: "ios_embedded",
            client_id: Some(66),
            api_key: "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",
            context: ContextClient {
                client_name: "IOS_MESSAGES_EXTENSION",
                client_version: "16.46",
                client_screen: None,
                device_model: Some("iPhone14,3"),
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static IOS_MUSIC: Client = Client {
            name: "ios_music",
            client_id: Some(26),
            api_key: "AIzaSyBAETezhkwP0ZWA02RsqT1zu78Fpt0bC_s",
            context: ContextClient {
                client_name: "IOS_MUSIC",
                client_version: "4.57",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        pub static IOS_CREATOR: Client = Client {
            name: "ios_creator",
            client_id: Some(15),
            api_key: "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",
            context: ContextClient {
                client_name: "IOS_CREATOR",
                client_version: "21.47",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
            user_agent: None,
            chrome_target: None,
            ff_target: None,
        };
        // all web formats require JS crypto handling for formats - currently not supported, but we need microformats
        pub static WEB: Client = Client {
            name: "web",
            client_id: Some(1),
            api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
            context: ContextClient {
                client_name: "WEB",
                client_version: "2.20211221.00.00",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: true,
            user_agent: Some(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome104",
                user_agent: None,
            }),
            ff_target: Some(ImpersonationTarget {
                target: "ff102",
                user_agent: None,
            }),
        };
        pub static WEB_EMBEDDED: Client = Client {
            name: "web_embedded",
            client_id: Some(56),
            api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
            context: ContextClient {
                client_name: "WEB_EMBEDDED_PLAYER",
                client_version: "1.20211215.00.01",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: true,
            user_agent: Some(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome104",
                user_agent: None,
            }),
            ff_target: Some(ImpersonationTarget {
                target: "ff102",
                user_agent: None,
            }),
        };
        pub static WEB_MUSIC: Client = Client {
            name: "web_music",
            client_id: Some(67),
            api_key: "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30",
            context: ContextClient {
                client_name: "WEB_REMIX",
                client_version: "1.20211213.00.00",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "music.youtube.com",
            js_needed: true,
            user_agent: Some(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome104",
                user_agent: None,
            }),
            ff_target: Some(ImpersonationTarget {
                target: "ff102",
                user_agent: None,
            }),
        };
        pub static WEB_CREATOR: Client = Client {
            name: "web_creator",
            client_id: Some(62),
            api_key: "AIzaSyBUPetSUmoZL-OhlxA7wSac5XinrygCqMo",
            context: ContextClient {
                client_name: "WEB_CREATOR",
                client_version: "1.20211220.02.00",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: true,
            user_agent: Some(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101 Firefox/105.0",
            ),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome104",
                user_agent: None,
            }),
            ff_target: Some(ImpersonationTarget {
                target: "ff102",
                user_agent: None,
            }),
        };
        pub static MWEB: Client = Client {
            name: "mweb",
            client_id: Some(2),
            api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
            context: ContextClient {
                client_name: "MWEB",
                client_version: "2.20211221.01.00",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "m.youtube.com",
            js_needed: true,
            user_agent: Some("Mozilla/5.0 (Linux; Android 10; LM-Q710(FGN)) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.5195.136 Mobile Safari/537.36"),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome99_android",
                user_agent: None,
            }),
            ff_target: Some(ImpersonationTarget {
                target: "ff102",
                user_agent: Some("Mozilla/5.0 (Android 13; Mobile; rv:102.0) Gecko/102.0 Firefox/102.0"),
            }),
        };
        pub static TV_EMBEDDED: Client = Client {
            name: "tv_embedded",
            client_id: Some(85),
            api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
            context: ContextClient {
                client_name: "TVHTML5_SIMPLY_EMBEDDED_PLAYER",
                client_version: "2.0",
                client_screen: None,
                device_model: None,
                hl: None,
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://www.youtube.com",
            }),
            host: "www.youtube.com",
            js_needed: true,
            user_agent: Some("Mozilla/5.0 (Fuchsia) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.5112.130 Safari/537.36 CrKey/1.56.500000"),
            chrome_target: Some(ImpersonationTarget {
                target: "chrome104",
                user_agent: Some("Mozilla/5.0 (Fuchsia) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.5112.130 Safari/537.36 CrKey/1.56.500000"),
            }),
            ff_target: None,
        };
    }

    #[derive(SmartDefault, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/player`
    pub struct Player<'a> {
        pub video_id: String,
        pub context: parts::Context<'a>,
        #[default = true]
        pub content_check_ok: bool,
        #[default = true]
        pub racy_check_ok: bool,
        pub playback_context: parts::PlaybackContext,
    }

    #[derive(SmartDefault, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/browse`
    pub struct Browse<'a> {
        /// Note: browse_id is NOT just the playlist ID, you might need to use /navigation/resolve_url
        pub browse_id: String,
        pub continuation: Option<String>,
        pub context: parts::Context<'a>,
        /// Possibly returned by /navigation/resolve_url
        pub params: Option<String>,
    }

    #[derive(SmartDefault, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/navigation/resolve_url`
    pub struct NavigationResolve<'a> {
        pub context: parts::Context<'a>,
        pub url: String,
    }
}

pub struct VideoList<T> {
    pub videos: Vec<T>,
    pub continuations: Vec<Continuation>,
}
