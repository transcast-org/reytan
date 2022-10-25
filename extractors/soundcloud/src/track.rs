use qstring::QString;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::url::Url;
use reytan_extractor_api::{
    async_trait, Extractable, Extraction, ExtractionContext, RecordingExtractor, URLMatcher,
};

use crate::common::{get_api_request, SOUNDCLOUD_API_DOMAINS, SOUNDCLOUD_USER_DOMAINS};
use crate::types::Track;

pub struct SoundcloudRE {}

impl SoundcloudRE {
    fn do_match(&self, url: &Url) -> (bool, Option<(String, Option<String>)>) {
        let segments: Vec<_> = url.path_segments().unwrap().collect();
        if let Some(h) = url.host_str() {
            if SOUNDCLOUD_USER_DOMAINS.contains(&h)
                && segments
                    .get(0)
                    .filter(|s| !["stations"].contains(s))
                    .is_some()
                && segments
                    .get(1)
                    .filter(|s| {
                        ["tracks", "albums", "sets", "reposts", "likes", "spotlight"].contains(s)
                    })
                    .is_some()
            {
                (true, None)
            } else if SOUNDCLOUD_API_DOMAINS.contains(&h)
                && segments.get(0) == Some(&"tracks")
                && segments
                    .get(1)
                    .unwrap_or(&"")
                    .chars()
                    .all(|c| "1234567890".contains(c))
            {
                (
                    true,
                    Some((
                        segments.get(1).unwrap().to_string(),
                        QString::new(url.query_pairs().collect())
                            .get("secret_token")
                            .map(str::to_string),
                    )),
                )
            } else {
                (false, None)
            }
        } else {
            (false, None)
        }
    }
}

impl URLMatcher for SoundcloudRE {
    fn match_extractor(&self, url: &Url) -> bool {
        Some(url)
            .filter(|u| match u.scheme() {
                "http" | "https" => true,
                _ => false,
            })
            .filter(|u| self.do_match(u).0)
            .is_some()
    }
}

#[async_trait]
impl RecordingExtractor for SoundcloudRE {
    async fn extract_recording(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        _extractable: &Extractable,
    ) -> Result<Extraction> {
        let track: Track = if let Some((track_id, secret_token)) = self.do_match(url).1 {
            get_api_request(
                ctx,
                &format!("/tracks/{}", &track_id),
                &mut QString::new(if let Some(token) = secret_token {
                    vec![("secret_token", token)]
                } else {
                    vec![]
                }),
            )
            .await?
        } else {
            get_api_request(
                ctx,
                "/resolve",
                &mut QString::new(vec![("url", url.as_str())]),
            )
            .await?
        };

        return Ok(track.into());
    }
}

#[cfg(test)]
mod tests {
    use super::SoundcloudRE;
    use reytan_extractor_api::url::Url;
    use reytan_extractor_api::{ExtractLevel, Extractable, ExtractionContext, RecordingExtractor};

    #[tokio::test]
    async fn test_extraction_basic() {
        let soundcloud = SoundcloudRE {};
        let ctx = ExtractionContext::new();
        let recording = soundcloud.extract_recording(&ctx, &Url::parse("https://soundcloud.com/goophouse/nyancrimew-this-video-game-has?in=goophouse/sets/goop-house-volume-7").unwrap(), &Extractable {
            metadata: ExtractLevel::Extended,
            playback: ExtractLevel::Extended,
        }).await.unwrap();
        let metadata = recording.metadata.unwrap();
        assert_eq!(metadata.id, "1294648321");
        assert_eq!(
            metadata.title,
            "nyancrimew - this video game has gender dysphoria"
        );
    }

    #[tokio::test]
    async fn test_extraction_secret_web() {
        let soundcloud = SoundcloudRE {};
        let ctx = ExtractionContext::new();
        let recording = soundcloud
            .extract_recording(
                &ctx,
                &Url::parse("https://soundcloud.com/jaimemf/youtube-dl-test-video-a-y-baw/s-8Pjrp")
                    .unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .unwrap();
        let metadata = recording.metadata.unwrap();
        assert_eq!(metadata.id, "123998367");
        assert_eq!(metadata.title, "Youtube - Dl Test Video \'\' Ä↭");
    }

    #[tokio::test]
    async fn test_extraction_secret_api() {
        let soundcloud = SoundcloudRE {};
        let ctx = ExtractionContext::new();
        let recording = soundcloud
            .extract_recording(
                &ctx,
                &Url::parse("https://api.soundcloud.com/tracks/123998367?secret_token=s-8Pjrp")
                    .unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .unwrap();
        let metadata = recording.metadata.unwrap();
        assert_eq!(metadata.id, "123998367");
        assert_eq!(metadata.title, "Youtube - Dl Test Video \'\' Ä↭");
    }
}
