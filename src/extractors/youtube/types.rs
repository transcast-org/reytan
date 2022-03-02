pub mod response {
    pub mod parts {
        use serde::Deserialize;
        use serde_aux::prelude::*;

        use crate::extractors::api::{self, MediaFormat, FormatBreed};
        
        #[derive(SmartDefault, Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Format {
            /// innertube format id
            pub itag: u16,
            /// url to download - not present in web
            pub url: Option<String>,
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

        impl From<Format> for MediaFormat {
            fn from(fmt: Format) -> MediaFormat {
                let breed = if fmt.mime_type.starts_with("audio/") {
                    FormatBreed::Audio
                // multiple codecs - "video/3gpp; codecs=\"mp4v.20.3, mp4a.40.2\""
                } else if fmt.mime_type.contains(", ") {
                    FormatBreed::AudioVideo
                } else {
                    FormatBreed::Video
                };
                MediaFormat {
                    id: fmt.itag.to_string(),
                    url: fmt.url.unwrap(),
                    video_details: if breed == FormatBreed::Video || breed == FormatBreed::AudioVideo {
                        Some(api::VideoDetails {
                            width: fmt.width,
                            height: fmt.height,
                            ..Default::default()
                        })
                    } else {
                        None
                    },
                    audio_details: if breed == FormatBreed::Audio || breed == FormatBreed::AudioVideo {
                        Some(api::AudioDetails {
                            channels: fmt.audio_channels,
                            ..Default::default()
                        })
                    } else {
                        None
                    },
                    breed,
                    ..Default::default()
                }
            }
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct StreamingData {
            // not present in ios responses
            pub formats: Option<Vec<Format>>,
            // not present in ios_creator responses
            pub adaptive_formats: Option<Vec<Format>>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlayabilityStatus {
            pub status: String,
            pub reason: Option<String>,
            pub reason_title: Option<String>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
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
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct MicroformatsWrapper {
            pub player_microformat_renderer: Option<Microformats>,
            pub microformat_data_renderer: Option<MicroformatsMusic>,
        }

        /// Microformats for web and web_* EXCEPT web_music
        #[derive(Deserialize, PartialEq, Debug)]
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
        #[derive(Deserialize, PartialEq, Debug)]
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

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct MicroformatsMusicVideoDetails {
            #[serde(deserialize_with = "deserialize_option_number_from_string")]
            #[serde(default)]
            pub duration_seconds: Option<u64>,
            pub external_video_id: Option<String>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct RunsWrapper {
            pub simple_text: Option<String>,
            pub runs: Option<Vec<RunsInner>>,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct RunsInner {
            pub text: String,
        }
    }

    use serde::Deserialize;

    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/player`
    pub struct Player {
        pub streaming_data: Option<parts::StreamingData>,
        pub playability_status: parts::PlayabilityStatus,
        pub video_details: parts::VideoDetails,
        pub microformat: Option<parts::MicroformatsWrapper>,
    }
}

pub mod request {
    pub mod parts {
        use serde::Serialize;

        #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ThirdParty <'a> {
            pub embed_url: &'a str,
        }

        #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ContextClient <'a> {
            pub client_name: &'a str,
            pub client_version: &'a str,
            pub client_screen: Option<&'a str>,
            pub device_model: Option<&'a str>,
            #[default = "en"]
            pub hl: &'a str,
            #[default = "UTC"]
            pub time_zone: &'a str,
            #[default = 0]
            pub utc_offset_minutes: u8,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct Context <'a> {
            pub client: ContextClient <'a>,
            pub third_party: Option<ThirdParty <'a>>,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct ContentPlaybackContext {
            #[default = "HTML5_PREF_WANTS"]
            html5_preference: String,
        }

        #[derive(SmartDefault, Serialize, Clone, Debug)]
        #[serde(rename_all = "camelCase")]
        pub struct PlaybackContext {
            content_playback_context: ContentPlaybackContext,
        }
    }

    use serde::Serialize;
    use smart_default::SmartDefault;

    #[derive(SmartDefault, Serialize, Clone, Copy, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct Client <'a> {
        pub name: &'a str,
        pub client_id: Option<u16>,
        #[default = "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8"]
        pub api_key: &'a str,
        pub context: parts::ContextClient <'a>,
        pub third_party: Option<parts::ThirdParty <'a>>,
        #[default = "www.youtube.com"]
        pub host: &'a str,
        pub js_needed: bool,
    }

    /// INNERTUBE_CLIENTS from yt-dlp: https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/youtube.py
    pub mod clients {
        use super::{Client, parts::{ContextClient, ThirdParty}};
        pub static ANDROID_MUSIC: Client = Client {
            name: "android_music",
            client_id: Some(21),
            api_key: "AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI",
            context: ContextClient {
                client_name: "ANDROID_MUSIC",
                client_version: "4.57",
                client_screen: None,
                device_model: None,
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
        };
        pub static ANDROID_AGEGATE: Client = Client {
            name: "android_agegate",
            client_id: Some(3),
            api_key: "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w",
            context: ContextClient {
                client_name: "ANDROID",
                client_version: "16.49",
                client_screen: Some("EMBED"),
                device_model: None,
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
        };
        pub static IOS_AGEGATE: Client = Client {
            name: "ios_agegate",
            client_id: Some(66),
            api_key: "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",
            context: ContextClient {
                client_name: "IOS_MESSAGES_EXTENSION",
                client_version: "16.46",
                client_screen: Some("EMBED"),
                device_model: Some("iPhone14,3"),
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: false,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: true,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: true,
        };
        pub static WEB_AGEGATE: Client = Client {
            name: "web_agegate",
            client_id: Some(1),
            api_key: "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",
            context: ContextClient {
                client_name: "WEB",
                client_version: "2.20211221.00.00",
                client_screen: Some("EMBED"),
                device_model: None,
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: Some(ThirdParty {
                embed_url: "https://google.com",
            }),
            host: "www.youtube.com",
            js_needed: true,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "music.youtube.com",
            js_needed: true,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: true,
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
                hl: "en",
                time_zone: "UTC",
                utc_offset_minutes: 0,
            },
            third_party: None,
            host: "www.youtube.com",
            js_needed: true,
        };
    }

    #[derive(SmartDefault, Serialize, Debug)]
    #[serde(rename_all = "camelCase")]
    /// `/youtubei/v1/player`
    pub struct Player <'a> {
        pub video_id: String,
        pub context: parts::Context <'a>,
        #[default = true]
        pub content_check_ok: bool,
        #[default = true]
        pub racy_check_ok: bool,
        pub playback_context: parts::PlaybackContext,
    }
}
