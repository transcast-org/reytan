use std::collections::HashSet;

use crate::extractors::api::{
    ExtractLevel, Extractable, Extraction, MediaFormat, MediaMetadata, MediaPlayback,
    RecordingExtractor, URLMatcher,
};
use crate::extractors::youtube::common::{
    innertube_request, YOUTUBE_HOSTS_MAIN, YOUTUBE_HOSTS_SHORT,
};
use crate::extractors::youtube::types::response::parts::Format;
use crate::extractors::youtube::types::{
    request::{self, clients},
    response::{self, parts::StreamingData},
};

use anyhow::Result;
use async_trait::async_trait;
use boa_engine::Context;
use once_cell::sync::Lazy;
use qstring::QString;
use regex::Regex;
use reqwest::{self, header, Client};
use url::Url;

pub struct YoutubeRE {}

impl YoutubeRE {
    async fn yti_player(
        &self,
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

impl URLMatcher for YoutubeRE {
    fn match_extractor(&self, url: &Url) -> bool {
        Some(url)
            .filter(|u| match u.scheme() {
                "http" | "https" => true,
                _ => false,
            })
            .filter(|u| {
                let host = u.host_str().unwrap();
                let first_segment = u.path_segments().unwrap().next().unwrap();
                (YOUTUBE_HOSTS_MAIN.contains(&host)
                    && ["watch", "video", "shorts"].contains(&first_segment))
                    || YOUTUBE_HOSTS_SHORT.contains(&host)
            })
            .is_some()
    }
}

impl YoutubeRE {
    fn get_id(&self, url: &Url) -> String {
        let host = url.host_str().unwrap();
        let mut segments = url.path_segments().unwrap();
        let first_segment = segments.next().unwrap();
        if YOUTUBE_HOSTS_MAIN.contains(&host) {
            if first_segment == "watch" {
                return url
                    .query_pairs()
                    .find(|(k, _)| k == "v")
                    .unwrap()
                    .1
                    .to_string();
            } else {
                return segments.next().unwrap().to_string();
            }
        } else {
            return first_segment.to_string();
        }
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

static WEB_PLAYER_RE: Lazy<Regex> =
    // excessive, leaves anything after the json
    Lazy::new(|| Regex::new(r"var ytInitialPlayerResponse\s*=\s*(\{.+);").unwrap());

static WEB_JS_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""jsUrl"\s*:\s*"(/s/player/([a-z0-9]+)/(?:player_ias\.vflset/[^/]+|player-plasma-ias-phone-[^/.]+\.vflset)/base\.js)""#)
        .unwrap()
});

static WEB_JS_SIG_FN_NAME_RE: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        // from yt-dlp
        r"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(",
        r"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(",
        r"\bm=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(h\.s\)\)",
        r"\bc&&\(c=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(c\)\)",
        r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*{\s*a\s*=\s*a\.split\(\s*""\s*\);[a-zA-Z0-9$]{2}\.[a-zA-Z0-9$]{2}\(a,\d+\)"#,
        r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
        r#"(?P<sig>[a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
    ].into_iter().map(Regex::new).flat_map(Result::ok).collect()
});

static WEB_JS_NCODE_FN_INITIAL_NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"&&\(b=a.get\("n"\)\)&&\(b=(?P<ncvar>[a-zA-Z0-9_$]{2,})(?:\[(?P<index>0)\])?\(b\)"#,
    )
    .unwrap()
});

impl YoutubeRE {
    async fn handle_sig(
        &self,
        http: &reqwest::Client,
        script_url: Url,
        streaming_data: &mut StreamingData,
    ) {
        let player_js = http
            .get(script_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let sig_fn_name = WEB_JS_SIG_FN_NAME_RE
            .iter()
            .find_map(|r| r.captures(&player_js))
            .unwrap()
            .name("sig")
            .unwrap()
            .as_str();
        dbg!(sig_fn_name);
        let sig_fn_match = Regex::new(&format!(
                    r#"(?:function\s+{0}|[{{;,]\s*{0}\s*=\s*function|(?:var|const|let)\s+{0}\s*=\s*function)\s*\((?P<args>[^)]*)\)\s*(?P<code>\{{\s*a\s*=\s*a\s*\.\s*split\s*\(\s*(?:""|'')\s*\)\s*;\s*(?P<mangler>[a-zA-Z0-9_$]{{2}})\s*\..+?}})"#,
                    regex::escape(sig_fn_name),
                )).unwrap().captures(&player_js).unwrap();
        let sig_fn_code = sig_fn_match.name("code").unwrap().as_str();
        let sig_fn_args = sig_fn_match.name("args").unwrap().as_str();
        let sig_fn_mangler_name = sig_fn_match.name("mangler").unwrap().as_str();
        dbg!(sig_fn_code);
        dbg!(sig_fn_args);
        dbg!(sig_fn_mangler_name);
        let sig_manglers = Regex::new(&format!(
            r#"(?s)(?:(?:var|const|let)\s+|[{{;,]\s*){0}\s*=\s*(?P<code>\{{.+?}}\s*}}\s*);"#,
            regex::escape(sig_fn_mangler_name),
        ))
        .unwrap()
        .captures(&player_js)
        .unwrap()
        .name("code")
        .unwrap()
        .as_str();
        dbg!(sig_manglers);
        let ncode_fn_init_name_match = WEB_JS_NCODE_FN_INITIAL_NAME_RE
            .captures(&player_js)
            .unwrap();
        let ncode_fn_name =
            if ncode_fn_init_name_match.name("index").is_some() {
                Regex::new(&format!(
                r#"(?:(?:var|const|let)\s+|[}};]\s*){}\s*=\s*\[\s*([a-zA-Z0-9_$]{{2,}})\s*\]\s*;"#,
                regex::escape(ncode_fn_init_name_match.name("ncvar").unwrap().as_str()),
            ))
            .unwrap().captures(&player_js).unwrap().get(1).unwrap().as_str()
            } else {
                ncode_fn_init_name_match.name("ncvar").unwrap().as_str()
            };
        let ncode_match = Regex::new(&format!(
            r#"(?s){0}\s*=\s*function\s*\((?P<args>[^)]*)\)(?P<code>\{{.+?return\s+b\.join\((?:""|'')\);?\}});"#,
            regex::escape(ncode_fn_name))).unwrap().captures(&player_js).unwrap();
        let ncode_fn_args = ncode_match.name("args").unwrap().as_str();
        let ncode_fn_code = ncode_match.name("code").unwrap().as_str();
        let js_payload = format!(
            "
            const {sig_fn_mangler_name}={sig_manglers};
            const sig=function({sig_fn_args}){sig_fn_code};
            const ncode=function({ncode_fn_args}){ncode_fn_code};
            "
        );
        let mut js_context = Context::default();
        js_context.eval(js_payload).unwrap();

        for formats in [
            streaming_data.formats.as_mut(),
            streaming_data.adaptive_formats.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            for format in formats {
                let mut url = if let Some(fmt_url_) = &format.url {
                    // format.url = ncode(fmt_url)
                    Url::parse(fmt_url_).unwrap()
                } else if let Some(sc) = &format.signature_cipher {
                    let args = QString::from(sc.as_str());
                    let sc_url_s = args.get("url").unwrap();
                    let mut sc_url = Url::parse(sc_url_s).unwrap();
                    if let Some(s) = args.get("s") {
                        // the param name under which the processed signature should be available
                        // format.url = ncode(decipher(sc))
                        let sig_r = js_context
                            .eval(format!("sig({})", serde_json::to_string(s).unwrap()))
                            .unwrap()
                            .to_string(&mut js_context)
                            .unwrap()
                            .to_string();

                        let word_signature = "signature".to_string(); // fuck lifetimes
                        let signature_param = args.get("sp").unwrap_or(&word_signature);
                        let mut sc_url_params = QString::from(sc_url.query().unwrap());
                        sc_url_params.add_pair((signature_param, sig_r));
                        sc_url.set_query(Some(&sc_url_params.to_string()));
                        sc_url
                    } else {
                        sc_url.clone()
                    }
                } else {
                    panic!();
                };

                let mut url_params = QString::from(url.query().unwrap());
                let original_n = url_params.get("n");
                let ncode_r = js_context
                    .eval(format!(
                        "ncode({})",
                        serde_json::to_string(&original_n).unwrap()
                    ))
                    .unwrap()
                    .to_string(&mut js_context)
                    .unwrap()
                    .to_string();
                url_params = QString::new(
                    url_params
                        .into_pairs()
                        .into_iter()
                        .filter(|(k, _)| k != "n")
                        .collect(),
                );
                url_params.add_pair(("n", ncode_r));
                url.set_query(Some(url_params.to_string().as_str()));

                format.url = Some(url.to_string());
            }
        }
    }
    async fn extract_player_web(
        &self,
        http: &reqwest::Client,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            client.user_agent.unwrap().parse().unwrap(),
        );
        let webpage = http
            .get(format!("https://{}/watch?v={id}", client.host))
            .headers(headers)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let script_path = WEB_JS_URL_RE
            .captures(&webpage)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let script_url = url::Url::join(
            &Url::parse(&format!("https://{}/", client.host)).unwrap(),
            script_path,
        )
        .unwrap();

        let player_json = WEB_PLAYER_RE
            .captures(&webpage)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();
        let mut player: response::Player = serde_json::Deserializer::from_str(player_json)
            .into_iter()
            .next()
            .unwrap()
            .unwrap();

        if player.playability_status.status == "OK" {
            if let Some(streaming_data) = player.streaming_data.as_mut() {
                self.handle_sig(http, script_url, streaming_data).await;
            }
        }

        return Ok(player);
    }

    async fn extract_player(
        &self,
        http: &reqwest::Client,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        if client.js_needed {
            self.extract_player_web(http, &id, client).await
        } else {
            self.yti_player(http, &id, client).await
        }
    }

    async fn get_player(
        &self,
        http: &reqwest::Client,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<response::Player> {
        let id = self.get_id(url);
        let mut players = HashSet::new();
        if wanted.metadata == ExtractLevel::Extended {
            let result = self.extract_player(http, &id, &clients::MWEB).await;
            if let Ok(player) = result {
                players.insert(player);
            }
        } else {
            let result = self.extract_player(http, &id, &clients::ANDROID).await;
            if let Ok(player) = result {
                players.insert(player);
            }
        }

        // if live, iOS has unique formats: https://github.com/TeamNewPipe/NewPipeExtractor/issues/680
        if wanted.playback == ExtractLevel::Extended
            && players.iter().any(|p| p.video_details.is_live)
        {
            let result = self.extract_player(http, &id, &clients::IOS).await;
            if let Ok(player) = result {
                players.insert(player);
            }
        }

        Ok(players
            .into_iter()
            .reduce(|mut prev, cur| {
                prev.microformat = prev.microformat.or(cur.microformat);
                if prev.playability_status.status != "OK" && cur.playability_status.status == "OK" {
                    prev.playability_status.status = "OK".to_string();
                }
                if let Some(prev_streaming_data) = prev.streaming_data.as_mut() {
                    if let Some(cur_streaming_data) = &cur.streaming_data {
                        if let Some(prev_formats) = &prev_streaming_data.adaptive_formats {
                            if let Some(cur_formats) = &cur_streaming_data.adaptive_formats {
                                let mut available_formats = HashSet::new();
                                prev_streaming_data.adaptive_formats = Some(
                                    prev_formats
                                        .into_iter()
                                        .chain(cur_formats)
                                        .map(Format::clone)
                                        .filter(|f| available_formats.insert(f.itag))
                                        .collect(),
                                );
                            }
                        } else {
                            prev_streaming_data.adaptive_formats =
                                cur_streaming_data.adaptive_formats.clone();
                        }

                        if let Some(prev_formats) = &prev_streaming_data.formats {
                            if let Some(cur_formats) = &cur_streaming_data.formats {
                                let mut available_formats = HashSet::new();
                                prev_streaming_data.formats = Some(
                                    prev_formats
                                        .into_iter()
                                        .chain(cur_formats)
                                        .map(Format::clone)
                                        .filter(|f| available_formats.insert(f.itag))
                                        .collect(),
                                );
                            }
                        } else {
                            prev_streaming_data.formats = cur_streaming_data.formats.clone();
                        }
                    }
                } else {
                    prev.streaming_data = prev.streaming_data.or(cur.streaming_data);
                }

                prev
            })
            .unwrap())
    }
}

#[async_trait]
impl RecordingExtractor for YoutubeRE {
    async fn extract_recording(
        &self,
        http: &reqwest::Client,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<Extraction> {
        let player = self.get_player(http, url, wanted).await?;
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

    use super::super::types::request::clients::{ANDROID_MUSIC, TV_EMBEDDED};
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
            .yti_player(&build_http(), "o6wtDPVkKqI", &TV_EMBEDDED)
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
                &Url::parse("https://youtu.be/KushW6zvazM").unwrap(),
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

    #[test]
    fn test_url_match_watch() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap());
        assert_eq!(url_match, true);
    }

    #[test]
    fn test_url_match_video() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/video/dQw4w9WgXcQ").unwrap());
        assert_eq!(url_match, true);
    }

    #[test]
    fn test_url_match_shorts() {
        let youtube = YoutubeRE {};
        let url_match = youtube
            .match_extractor(&Url::parse("https://www.youtube.com/shorts/dQw4w9WgXcQ").unwrap());
        assert_eq!(url_match, true);
    }

    #[test]
    fn test_url_match_shortener() {
        let youtube = YoutubeRE {};
        let url_match =
            youtube.match_extractor(&Url::parse("https://youtu.be/dQw4w9WgXcQ").unwrap());
        assert_eq!(url_match, true);
    }
}
