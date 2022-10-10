use anyhow::Result;
use async_trait::async_trait;
use nipper::Document;
use reytan_extractor_api::reqwest::header;
use reytan_extractor_api::{
    AudioDetails, Extractable, Extraction, ExtractionContext, FormatBreed, MediaFormat,
    MediaMetadata, MediaPlayback, RecordingExtractor, URLMatcher,
};
use url::Url;

use super::common::{_is_bandcamp, _path_is};
use super::types::web_fragments::DataTralbum;

pub struct BandcampRE {}

impl URLMatcher for BandcampRE {
    fn match_extractor(&self, url: &Url) -> bool {
        Some(url)
            .filter(|u| match u.scheme() {
                "http" | "https" => true,
                _ => false,
            })
            // url path starts with `/track/`
            .filter(|u| _path_is(u, "track"))
            .filter(_is_bandcamp)
            .is_some()
    }
}

#[async_trait]
impl RecordingExtractor for BandcampRE {
    async fn extract_recording(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
        _wanted: &Extractable,
    ) -> Result<Extraction> {
        let webpage = ctx
            .http
            .get(url.clone())
            .header(
                header::USER_AGENT,
                "Mozilla/5.0 (Linux x86_64; rv:102.0) Gecko/20100101 Firefox/102.0",
            )
            .send()
            .await?
            .text()
            .await?;
        let document = Document::from(&webpage);
        let dtralbum = document
            .select("script[data-tralbum]")
            .attr("data-tralbum")
            .unwrap()
            .to_string();
        let tralbum: DataTralbum = serde_json::from_str(&dtralbum).expect("data-tralbum");
        // for some reason, it's an array with one item
        let trackinfo = tralbum.trackinfo.get(0).expect("trackinfo");
        Ok(Extraction {
            metadata: Some(MediaMetadata {
                id: tralbum.url,
                title: trackinfo.title.clone(),
                ..Default::default()
            }),
            playback: Some(MediaPlayback {
                formats: {
                    trackinfo
                        .file
                        .keys()
                        .into_iter()
                        .map(|quality| MediaFormat {
                            id: quality.to_string(),
                            breed: FormatBreed::Audio,
                            url: trackinfo.file.get(quality).unwrap().to_string(),
                            audio_details: Some(AudioDetails {
                                ..Default::default()
                            }),
                            ..Default::default()
                        })
                        .collect()
                },
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use reytan_extractor_api::{
        ExtractLevel, Extractable, ExtractionContext, RecordingExtractor, URLMatcher,
    };
    use url::Url;

    use super::BandcampRE;

    #[test]
    fn match_track_url() {
        let bandcamp = BandcampRE {};
        let mtch = bandcamp
            .match_extractor(
                &Url::parse("http://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny/?utm_source=your_mom#aoeu")
                    .unwrap());
        assert_eq!(mtch, true);
    }

    #[tokio::test]
    async fn do_fetch_tralbum() {
        let bandcamp = BandcampRE {};
        let recording = bandcamp
            .extract_recording(
                &ExtractionContext::new(),
                &Url::parse("https://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny")
                    .unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .expect("extraction");
        let metadata = recording.metadata.expect("metadata");
        assert_eq!(metadata.title, "Make that Skirt go Spinny");
        let playback = recording.playback.expect("playback");
        assert_eq!(playback.formats.len(), 1);
    }

    #[tokio::test]
    async fn do_fetch_album_track() {
        let bandcamp = BandcampRE {};
        let recording = bandcamp
            .extract_recording(
                &ExtractionContext::new(),
                &Url::parse("https://penelopescott.bandcamp.com/track/r-t-2").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .expect("extraction");
        let metadata = recording.metadata.expect("metadata");
        assert_eq!(metadata.title, "Rät");
        let playback = recording.playback.expect("playback");
        assert_eq!(playback.formats.len(), 1);
    }
}
