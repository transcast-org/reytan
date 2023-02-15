use std::collections::{HashMap, HashSet};
use std::time::Duration;

use super::common::{innertube_request, YOUTUBE_HOSTS_MAIN, YOUTUBE_HOSTS_SHORT};
use super::types::request::{self, clients};
use super::types::response;
use super::types::response::parts::{Format, StreamingData};

#[cfg(feature = "allow_js")]
use boa_engine::Context as JSContext;
#[cfg(feature = "allow_js")]
use qstring::QString;
#[cfg(feature = "allow_js")]
use regex::Regex;
#[cfg(feature = "allow_js")]
use reytan_extractor_api::anyhow::Error;
#[cfg(feature = "allow_js")]
use serde::{Deserialize, Serialize};

use once_cell::sync::Lazy;
use reytan_extractor_api::anyhow::{bail, Result};
use reytan_extractor_api::url::Url;
use reytan_extractor_api::{
    async_trait, chrono, uri, ExtractLevel, Extractable, Extraction, ExtractionContext, LiveStatus,
    MediaFormatEstablished, MediaMetadata, NewExtractor, RecordingExtractor, URLMatcher, Utc,
};

pub struct YoutubeRE {}

impl NewExtractor for YoutubeRE {
    fn new() -> Self {
        YoutubeRE {}
    }
}

impl YoutubeRE {
    async fn yti_player(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client_: &request::Client<'_>,
        sts: Option<u32>,
    ) -> Result<response::Player> {
        let mut client = client_.clone();
        let hl = &ctx
            .locales
            .first()
            .cloned()
            .unwrap_or_else(|| "en".to_string())[0..2];
        client.context.hl = Some(hl);
        let json = request::Player {
            video_id: id.to_string(),
            context: request::parts::Context {
                client: client.context,
                third_party: client.third_party,
            },
            playback_context: request::parts::PlaybackContext {
                content_playback_context: request::parts::ContentPlaybackContext {
                    signature_timestamp: sts,
                    ..Default::default()
                },
            },
            ..Default::default()
        };
        innertube_request(
            ctx,
            &format!("player (as {})", client.name),
            &client,
            "player",
            json,
        )
        .await
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

fn parse_formats(strm: StreamingData) -> Vec<MediaFormatEstablished> {
    let mut fmts: Vec<MediaFormatEstablished> = vec![];
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
    if let Some(formats) = strm.hls_formats {
        for fmt in formats {
            fmts.push(fmt.into());
        }
    }
    fmts
}

#[cfg(feature = "allow_js")]
static WEB_PLAYER_RE: Lazy<Regex> =
    // excessive, leaves anything after the json
    Lazy::new(|| Regex::new(r"var ytInitialPlayerResponse\s*=\s*(\{.+);").unwrap());

#[cfg(feature = "allow_js")]
static WEB_JS_URL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#""jsUrl"\s*:\s*"(/s/player/([a-z0-9]+)/(?:player_ias\.vflset/[^/]+|player-plasma-ias-phone-[^/.]+\.vflset)/base\.js)""#)
        .unwrap()
});

#[cfg(feature = "allow_js")]
static WEB_JS_SIG_FN_NAME_RE: Lazy<Vec<Regex>> = Lazy::new(|| {
    [
        // from yt-dlp
        r"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(",
        r"\b[a-zA-Z0-9]+\s*&&\s*[a-zA-Z0-9]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*(?P<sig>[a-zA-Z0-9$]+)\(",
        r"\bm=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(h\.s\)\)",
        r"\bc&&\(c=(?P<sig>[a-zA-Z0-9$]{2,})\(decodeURIComponent\(c\)\)",
        r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\);[a-zA-Z0-9$]{2}\.[a-zA-Z0-9$]{2}\(a,\d+\)"#,
        r#"(?:\b|[^a-zA-Z0-9$])(?P<sig>[a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
        r#"(?P<sig>[a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
    ].into_iter().map(Regex::new).map(Result::unwrap).collect()
});

#[cfg(feature = "allow_js")]
static WEB_JS_NCODE_FN_INITIAL_NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"&&\(b=a.get\("n"\)\)&&\(b=(?P<ncvar>[a-zA-Z0-9_$]{2,})(?:\[(?P<index>0)\])?\(b\)"#,
    )
    .unwrap()
});

#[cfg(feature = "allow_js")]
static WEB_STS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[{,]"STS"\s*:\s*([0-9]{5})[,}]"#).unwrap());

#[cfg(feature = "allow_js")]
static WEB_JS_STS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"[{,]['"]?signatureTimestamp['"]?\s*:\s*(\d{5})\s*[},]"#).unwrap());

#[derive(PartialEq, Eq)]
enum PlayabilityCategory {
    /// playable, according to youtube
    Ok,
    /// youtube hates this client
    AgeGate,
    /// the video has not been published yet
    NotYet,
    /// youtube has a skill issue (unplayable globally, or geo gate)
    HostSkillIssue,
    /// that's on us
    ClientSkillIssue,
}
static PLAYABILITY_STATUS_TYPE: Lazy<HashMap<String, PlayabilityCategory>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // self-explanatory
    map.insert("OK".to_string(), PlayabilityCategory::Ok);

    // "Sign in to confirm your age. This video may be inappropriate for some users."
    map.insert("LOGIN_REQUIRED".to_string(), PlayabilityCategory::AgeGate);

    map.insert(
        "LIVE_STREAM_OFFLINE".to_string(),
        PlayabilityCategory::NotYet,
    );

    // "We're processing this video. Check back later."
    // "The uploader has not made this video available in your country"
    map.insert(
        "UNPLAYABLE".to_string(),
        PlayabilityCategory::HostSkillIssue,
    );
    // "This video is private",
    // "This video is no longer available due to a copyright claim by WMG",
    // "This video is no longer available because the YouTube account associated with this video has been closed."
    map.insert("ERROR".to_string(), PlayabilityCategory::HostSkillIssue);

    map.insert(
        "CONTENT_CHECK_REQUIRED".to_string(),
        PlayabilityCategory::ClientSkillIssue,
    );
    // [when the user is logged in] "This video may be inappropriate for some users."
    map.insert(
        "AGE_CHECK_REQUIRED".to_string(),
        PlayabilityCategory::ClientSkillIssue,
    );

    // internal reytan error
    map.insert(
        "REYTAN_FAILED_SIGNATURE".to_string(),
        PlayabilityCategory::ClientSkillIssue,
    );
    // cannot handle signatures with allow_js feature disabled
    map.insert(
        "REYTAN_NO_ALLOW_JS".to_string(),
        PlayabilityCategory::ClientSkillIssue,
    );

    map
});

#[cfg(feature = "allow_js")]
static WEB_JS_FUNCTIONS_POOL: &'static str = "youtube_js_player_fns";

#[cfg(feature = "allow_js")]
#[derive(Serialize, Deserialize, Default)]
struct SigDefinition {
    pub sig_code: String,
    pub ncode_code: String,
    pub js_sts: Option<u32>,
}

impl YoutubeRE {
    #[cfg(feature = "allow_js")]
    async fn get_js_functions(
        &self,
        ctx: &ExtractionContext,
        (script_url, script_hash): (Url, &str),
    ) -> Result<SigDefinition> {
        use reytan_extractor_api::Request;

        if let Ok(Some(functions)) = ctx
            .cache
            .get::<SigDefinition>(WEB_JS_FUNCTIONS_POOL, &script_hash)
            .await
        {
            return Ok(functions);
        }

        let player_js = ctx
            .get_body(
                &format!("js player {}", script_hash),
                Request::get(uri(script_url)).body(())?,
            )
            .await?;
        let sig_fn_name = WEB_JS_SIG_FN_NAME_RE
            .iter()
            .find_map(|r| r.captures(&player_js))
            .unwrap()
            .name("sig")
            .unwrap()
            .as_str();
        let sig_fn_match = Regex::new(&format!(
                    r#"(?:function\s+{0}|[{{;,]\s*{0}\s*=\s*function|(?:var|const|let)\s+{0}\s*=\s*function)\s*\((?P<args>[^)]*)\)\s*(?P<code>\{{\s*a\s*=\s*a\s*\.\s*split\s*\(\s*(?:""|'')\s*\)\s*;\s*(?P<mangler>[a-zA-Z0-9_$]{{2}})\s*\..+?}})"#,
                    regex::escape(sig_fn_name),
                )).unwrap().captures(&player_js).unwrap();
        let sig_fn_code = sig_fn_match.name("code").unwrap().as_str();
        let sig_fn_args = sig_fn_match.name("args").unwrap().as_str();
        let sig_fn_mangler_name = sig_fn_match.name("mangler").unwrap().as_str();
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
        let js_sts = WEB_JS_STS_RE
            .captures(&player_js)
            .map(|c| c.get(1).unwrap().as_str().parse().unwrap());
        let js_payload = SigDefinition {
            sig_code: format!(
                "
                const {sig_fn_mangler_name}={sig_manglers};
                const sig=function({sig_fn_args}){sig_fn_code};"
            ),
            ncode_code: format!("const ncode=function({ncode_fn_args}){ncode_fn_code};"),
            js_sts,
        };

        ctx.cache
            .set(WEB_JS_FUNCTIONS_POOL, &script_hash, &js_payload)
            .await?;

        Ok(js_payload)
    }

    #[cfg(feature = "allow_js")]
    async fn handle_sig(
        &self,
        _ctx: &ExtractionContext,
        js_payload: SigDefinition,
        streaming_data: &mut StreamingData,
    ) -> Result<()> {
        let mut js_context = JSContext::default();
        js_context
            .eval(&js_payload.sig_code)
            .map_err(|e| Error::msg(e.to_string(&mut js_context).unwrap().to_string()))?;
        js_context
            .eval(&js_payload.ncode_code)
            .map_err(|e| Error::msg(e.to_string(&mut js_context).unwrap().to_string()))?;

        for formats in [
            streaming_data.formats.as_mut(),
            streaming_data.adaptive_formats.as_mut(),
            streaming_data.hls_formats.as_mut(),
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
                            .map_err(|e| {
                                Error::msg(e.to_string(&mut js_context).unwrap().to_string())
                            })?
                            .to_string(&mut js_context)
                            .map_err(|e| {
                                Error::msg(e.to_string(&mut js_context).unwrap().to_string())
                            })?
                            .to_string();

                        let signature_param = args.get("sp").unwrap_or("signature");
                        let mut sc_url_params = QString::from(sc_url.query().unwrap());
                        sc_url_params.add_pair((signature_param, sig_r));
                        sc_url.set_query(Some(&sc_url_params.to_string()));
                        sc_url
                    } else {
                        sc_url.clone()
                    }
                } else {
                    bail!("neither of url or signatureCipher found in format");
                };

                let mut url_params = QString::from(url.query().unwrap());
                if let Some(original_n) = url_params.get("n") {
                    let ncode_r = js_context
                        .eval(format!(
                            "ncode({})",
                            serde_json::to_string(original_n).unwrap()
                        ))
                        .map_err(|e| Error::msg(e.to_string(&mut js_context).unwrap().to_string()))?
                        .to_string(&mut js_context)
                        .map_err(|e| Error::msg(e.to_string(&mut js_context).unwrap().to_string()))?
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
                }

                format.url = Some(url.to_string());
            }
        }

        Ok(())
    }
    #[cfg(feature = "allow_js")]
    async fn decode_formats(
        &self,
        ctx: &ExtractionContext,
        client: &request::Client<'_>,
        (sig_definition, script_hash): (SigDefinition, &str),
        mut player: response::Player,
    ) -> Result<response::Player> {
        if player.playability_status.status == "OK" {
            if let Some(streaming_data) = player.streaming_data.as_mut() {
                match self.handle_sig(ctx, sig_definition, streaming_data).await {
                    Ok(_) => {}
                    Err(e) => {
                        player.streaming_data = None;
                        player.playability_status.status = "REYTAN_FAILED_SIGNATURE".to_string();
                        player.playability_status.reason_title = Some(format!(
                            "Failed handling signatures (client: {}, player: {})",
                            client.name, script_hash
                        ));
                        player.playability_status.reason = Some(e.to_string());
                    }
                }
            }
        }

        Ok(player)
    }
    #[cfg(feature = "allow_js")]
    async fn extract_js_sts_and_player_web(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<((Url, String), Option<u32>, Option<response::Player>)> {
        use reytan_extractor_api::{header, Request};

        let is_embed = client.name.ends_with("_embedded");
        let mut request = Request::get(format!(
            "https://{}/{}{id}",
            client.host,
            if is_embed { "embed/" } else { "watch?v=" }
        ));
        if let Some(user_agent) = client.user_agent {
            request = request.header(header::USER_AGENT, user_agent);
        }
        let webpage = ctx
            .get_body(
                if is_embed { "embed page" } else { "watch page" },
                request.body(())?,
            )
            .await?;

        let script_match = WEB_JS_URL_RE.captures(&webpage).unwrap();
        let script_path = script_match.get(1).unwrap().as_str();
        let script_hash = script_match.get(2).unwrap().as_str();
        let script_url = Url::join(
            &Url::parse(&format!("https://{}/", client.host)).unwrap(),
            script_path,
        )
        .unwrap();

        let sts = WEB_STS_RE
            .captures(&webpage)
            .map(|c| c.get(1).unwrap().as_str().parse().unwrap());

        let player: Option<response::Player> =
            WEB_PLAYER_RE.captures(&webpage).map(|player_json| {
                serde_json::Deserializer::from_str(player_json.get(1).unwrap().as_str())
                    .into_iter()
                    .next()
                    .unwrap()
                    .unwrap()
            });

        Ok(((script_url, script_hash.to_string()), sts, player))
    }
    #[cfg(feature = "allow_js")]
    async fn extract_player_web(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        let ((script_url, script_hash), _, maybe_player) =
            self.extract_js_sts_and_player_web(ctx, id, client).await?;

        let js_payload = self
            .get_js_functions(ctx, (script_url, &script_hash))
            .await?;

        let mut player = maybe_player.unwrap();

        player = self
            .decode_formats(ctx, client, (js_payload, &script_hash), player)
            .await?;

        return Ok(player);
    }
    #[cfg(feature = "allow_js")]
    async fn extract_player_embedded(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        let ((script_url, script_hash), sts_web, maybe_player) =
            self.extract_js_sts_and_player_web(ctx, id, client).await?;

        let js_payload = self
            .get_js_functions(ctx, (script_url, &script_hash))
            .await?;

        // the player on the webpage is WEB_EMBEDDED client, might not the one we want
        let mut player = if client.name != clients::WEB_EMBEDDED.name || maybe_player.is_none() {
            self.yti_player(ctx, id, client, sts_web.or(js_payload.js_sts))
                .await
                .unwrap()
        } else {
            maybe_player.unwrap()
        };

        player = self
            .decode_formats(ctx, client, (js_payload, &script_hash), player)
            .await?;

        return Ok(player);
    }

    #[cfg(feature = "allow_js")]
    async fn extract_player(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        if client.js_needed {
            if client.name.ends_with("_embedded") {
                self.extract_player_embedded(ctx, &id, client).await
            } else {
                self.extract_player_web(ctx, &id, client).await
            }
        } else {
            self.yti_player(ctx, &id, client, None).await
        }
    }

    #[cfg(not(feature = "allow_js"))]
    async fn extract_player(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'_>,
    ) -> Result<response::Player> {
        let mut player_r = self.yti_player(ctx, &id, client, None).await;

        // if JS is needed for handling the signatures, we cannot handle them
        if client.js_needed {
            if let Ok(player) = player_r.as_mut() {
                player.streaming_data = None;
                player.playability_status.status = "REYTAN_NO_ALLOW_JS".to_string();
                player.playability_status.reason_title =
                    Some("Cannot handle signatures".to_string());
                player.playability_status.reason =
                    Some("reytan was built without the JS interpreter".to_string());
            }
        }

        player_r
    }

    async fn attempt_client<'a>(
        &self,
        players: &mut HashSet<response::Player>,
        attempted_clients: &mut HashSet<&'a str>,
        ctx: &ExtractionContext,
        id: &str,
        client: &request::Client<'a>,
    ) {
        if attempted_clients.insert(&client.name) {
            let result = self.extract_player(ctx, id, client).await;
            if let Ok(player) = result {
                players.insert(player);
            }
        }
    }

    async fn get_players(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<(response::Player, HashSet<response::Player>)> {
        let id = self.get_id(url);
        let mut players = HashSet::new();
        let mut attempted_clients = HashSet::new();

        self.attempt_client(
            &mut players,
            &mut attempted_clients,
            ctx,
            &id,
            &match () {
                // WEB gets more metadata at the expense of JS signatures existing
                _ if wanted.metadata == ExtractLevel::Extended => clients::WEB,
                // IOS hates open media standards and returns avc1/mp4a only
                _ if wanted.playback == ExtractLevel::None => clients::IOS,
                // simple choice
                _ => clients::ANDROID,
            },
        )
        .await;

        // if the previous fetching of playback failed, try ANDROID,
        // also ignore if the error was agegate
        if players.len() == 0
            || (wanted.playback != ExtractLevel::None
                && !players.iter().all(|p| {
                    [PlayabilityCategory::Ok, PlayabilityCategory::AgeGate]
                        .into_iter()
                        .any(|s| {
                            Some(&s) == PLAYABILITY_STATUS_TYPE.get(&p.playability_status.status)
                        })
                }))
        {
            self.attempt_client(
                &mut players,
                &mut attempted_clients,
                ctx,
                &id,
                &clients::ANDROID,
            )
            .await;
        }

        // TV_EMBEDDED is known to get age-gated videos without logging in: https://github.com/yt-dlp/yt-dlp/pull/3233
        if cfg!(feature = "allow_js")
            && wanted.playback != ExtractLevel::None
            && players.iter().any(|p| {
                PLAYABILITY_STATUS_TYPE.get(&p.playability_status.status)
                    == Some(&PlayabilityCategory::AgeGate)
            })
        {
            self.attempt_client(
                &mut players,
                &mut attempted_clients,
                ctx,
                &id,
                &clients::TV_EMBEDDED,
            )
            .await;
        }

        // if live, iOS has unique formats: https://github.com/TeamNewPipe/NewPipeExtractor/issues/680
        if wanted.playback == ExtractLevel::Extended
            && players.iter().any(|p| p.video_details.is_live)
        {
            self.attempt_client(
                &mut players,
                &mut attempted_clients,
                ctx,
                &id,
                &clients::IOS,
            )
            .await;
        }

        match players.clone().into_iter().reduce(|mut prev, cur| {
            prev.microformat = prev.microformat.or(cur.microformat);
            if prev.playability_status.status != "OK" && cur.playability_status.status == "OK" {
                prev.playability_status.status = "OK".to_string();
            }
            if let Some(prev_streaming_data) = prev.streaming_data.as_mut() {
                if let Some(cur_streaming_data) = &cur.streaming_data {
                    let mut available_formats = HashSet::new();

                    let mut merge_formats =
                        |prev_formats: &Vec<Format>, cur_formats: &Vec<Format>| {
                            prev_formats
                                .into_iter()
                                .chain(cur_formats)
                                .map(Format::clone)
                                .filter(|f| available_formats.insert(f.itag))
                                .collect()
                        };

                    if let Some((prev_formats, cur_formats)) = prev_streaming_data
                        .adaptive_formats
                        .as_ref()
                        .zip(cur_streaming_data.adaptive_formats.as_ref())
                    {
                        prev_streaming_data.adaptive_formats =
                            Some(merge_formats(prev_formats, cur_formats));
                    } else if cur_streaming_data.adaptive_formats.is_some() {
                        prev_streaming_data.adaptive_formats =
                            cur_streaming_data.adaptive_formats.clone();
                    }

                    if let Some((prev_formats, cur_formats)) = prev_streaming_data
                        .formats
                        .as_ref()
                        .zip(cur_streaming_data.formats.as_ref())
                    {
                        prev_streaming_data.formats =
                            Some(merge_formats(prev_formats, cur_formats));
                    } else if cur_streaming_data.formats.is_some() {
                        prev_streaming_data.formats = cur_streaming_data.formats.clone();
                    }

                    if let Some((prev_formats, cur_formats)) = prev_streaming_data
                        .hls_formats
                        .as_ref()
                        .zip(cur_streaming_data.hls_formats.as_ref())
                    {
                        prev_streaming_data.formats =
                            Some(merge_formats(prev_formats, cur_formats));
                    } else if cur_streaming_data.formats.is_some() {
                        prev_streaming_data.hls_formats = cur_streaming_data.hls_formats.clone();
                    }
                }
            } else {
                prev.streaming_data = cur.streaming_data;
            }

            prev
        }) {
            Some(player) => Ok((player, players)),
            None => bail!("no players fetched successfully"),
        }
    }
}

#[async_trait]
impl RecordingExtractor for YoutubeRE {
    async fn extract_recording(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<Extraction> {
        let (player, players) = self.get_players(ctx, url, wanted).await?;
        let fmts = if let Some(stream) = player.streaming_data {
            parse_formats(stream)
        } else {
            Vec::new()
        };
        Ok(Extraction {
            metadata: MediaMetadata {
                id: player.video_details.video_id,
                title: player.video_details.title,
                description: player.video_details.short_description,
                duration: player
                    .video_details
                    .length_seconds
                    // on livestreams, duration always equals 0
                    .filter(|_| !player.video_details.is_live)
                    .map(Duration::from_secs),
                view_count: player.video_details.view_count,
                live_status: if player.video_details.is_live {
                    Some(LiveStatus::IsLive)
                } else if player.video_details.is_live_content {
                    Some(LiveStatus::WasLive)
                } else {
                    Some(LiveStatus::NotLive)
                },
                published_time: player
                    .microformat
                    .as_ref()
                    .map(|w| {
                        if let Some(m) = &w.player_microformat_renderer {
                            &m.publish_date
                        } else if let Some(m) = &w.microformat_data_renderer {
                            &m.publish_date
                        } else {
                            &None
                        }
                    })
                    .map(|o| o.as_ref())
                    .flatten()
                    .map(|d| chrono::DateTime::parse_from_rfc2822(&d))
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                created_time: player
                    .microformat
                    .as_ref()
                    .map(|w| {
                        if let Some(m) = &w.player_microformat_renderer {
                            &m.upload_date
                        } else if let Some(m) = &w.microformat_data_renderer {
                            &m.upload_date
                        } else {
                            &None
                        }
                    })
                    .map(|o| o.as_ref())
                    .flatten()
                    .map(|d| chrono::DateTime::parse_from_rfc2822(&d))
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                age_limit: players
                    .iter()
                    .any(|p| {
                        PLAYABILITY_STATUS_TYPE.get(&p.playability_status.status)
                            == Some(&PlayabilityCategory::AgeGate)
                    })
                    .then_some(18)
                    .or(Some(0)),
                ..Default::default()
            },
            established_formats: fmts,
            established_subtitles: player
                .captions
                .map(|w| w.player_captions_tracklist_renderer.into())
                .unwrap_or_else(|| Vec::new()),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use reytan_extractor_api::url::Url;
    use reytan_extractor_api::{
        ExtractLevel, Extractable, ExtractionContext, FormatBreed, LiveStatus, MediaFormatURL,
        RecordingExtractor, URLMatcher,
    };

    use super::super::types::request::clients::ANDROID_MUSIC;
    use super::YoutubeRE;

    #[tokio::test]
    async fn do_yti_player_protected() {
        let youtube = YoutubeRE {};
        let response = youtube
            .yti_player(
                &ExtractionContext::new().unwrap(),
                "KushW6zvazM",
                &ANDROID_MUSIC,
                None,
            )
            .await
            .expect("yti player");
        assert_eq!(
            response.playability_status.status, "OK",
            "playability status"
        );
        assert_ne!(response.streaming_data, None, "no streaming data");
    }

    // agegate can only be circumvented with JS support
    #[cfg(feature = "allow_js")]
    #[tokio::test]
    async fn do_extract_agegate() {
        let youtube = YoutubeRE {};
        let response = youtube
            .extract_recording(
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://www.youtube.com/video/Tq92D6wQ1mg").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Extended,
                    ..Default::default()
                },
            )
            .await
            .expect("extracted player");
        let meta = response.metadata;
        assert_eq!(meta.title, "[MMD] Adios - EVERGLOW [+Motion DL]");
        assert_eq!(meta.live_status, Some(LiveStatus::NotLive));
        assert_eq!(meta.age_limit, Some(18));
        let formats = response.established_formats;
        assert!(formats.len() > 0);
        let f251 = formats
            .into_iter()
            .find(|f| f.details.id == "251")
            .expect("format 251");
        assert_eq!(f251.details.breed, FormatBreed::Audio);
        assert_eq!(f251.details.video_details, None);
        let audio = f251.details.audio_details.expect("251 audio details");
        assert_eq!(audio.channels.unwrap(), 2);
    }

    #[tokio::test]
    async fn do_extract_protected() {
        let youtube = YoutubeRE {};
        let response = youtube
            .extract_recording(
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://youtu.be/KushW6zvazM").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                    ..Default::default()
                },
            )
            .await
            .expect("player response");
        let meta = response.metadata;
        assert_eq!(meta.title, "DECO*27 - ゴーストルール feat. 初音ミク");
        assert_eq!(meta.live_status, Some(LiveStatus::NotLive));
        assert_eq!(meta.age_limit, Some(0));
        let formats = response.established_formats;
        assert!(formats.len() > 0);
        let f251 = formats
            .into_iter()
            .find(|f| f.details.id == "251")
            .expect("format 251");
        assert_eq!(f251.details.breed, FormatBreed::Audio);
        assert_eq!(f251.details.video_details, None);
        let audio = f251.details.audio_details.expect("251 audio details");
        assert_eq!(audio.channels.unwrap(), 2);
        match f251.url {
            MediaFormatURL::HTTP(u, _) => {
                assert!(u.host_str().unwrap().ends_with(".googlevideo.com"))
            }
            _ => panic!("251 should return HTTP URL"),
        }
    }

    #[tokio::test]
    async fn do_extract_live() {
        let youtube = YoutubeRE {};
        let response = youtube
            .extract_recording(
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://www.youtube.com/watch?v=jfKfPfyJRdk").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let meta = response.metadata;
        assert_eq!(meta.title, "lofi hip hop radio - beats to relax/study to");
        assert_eq!(meta.live_status, Some(LiveStatus::IsLive));
    }

    #[tokio::test]
    async fn do_extract_subtitles() {
        let youtube = YoutubeRE {};
        let response = youtube
            .extract_recording(
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://www.youtube.com/watch?v=UnIhRpIT7nc").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Basic,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let meta = response.metadata;
        assert_eq!(meta.title, "稲葉曇『ラグトレイン』Vo. 歌愛ユキ");
        assert_eq!(meta.live_status, Some(LiveStatus::NotLive));
        let subtitles = response.established_subtitles;
        // 3 languages, 6 formats
        assert_eq!(subtitles.len(), 3 * 6);
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

    #[cfg(feature = "allow_js")]
    #[test]
    fn test_regexes_compile() {
        for re in [
            &super::WEB_PLAYER_RE,
            &super::WEB_JS_URL_RE,
            &super::WEB_JS_NCODE_FN_INITIAL_NAME_RE,
            &super::WEB_STS_RE,
            &super::WEB_JS_STS_RE,
        ] {
            re.as_str();
        }
        let _ = &super::WEB_JS_SIG_FN_NAME_RE
            .iter()
            .map(regex::Regex::as_str);
    }
}
