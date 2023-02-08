use std::time::Duration;

use nipper::Document;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::{
    async_trait, chrono, header, uri, AudioDetails, Extractable, Extraction, ExtractionContext,
    FormatBreed, HTTPDownloadOptions, MediaFormatDetails, MediaFormatEstablished, MediaFormatURL,
    MediaMetadata, NewExtractor, RecordingExtractor, Request, URLMatcher, Url, Utc,
};

use super::common::{_is_bandcamp, _path_is};
use super::types::web_fragments::DataTralbum;

pub struct BandcampRE {}

impl NewExtractor for BandcampRE {
    fn new() -> Self {
        BandcampRE {}
    }
}

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
            .get_body(
                "webpage",
                Request::get(uri(url.clone()))
                    .header(
                        header::USER_AGENT,
                        "Mozilla/5.0 (Linux x86_64; rv:102.0) Gecko/20100101 Firefox/102.0",
                    )
                    .body(())?,
            )
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
            metadata: MediaMetadata {
                id: tralbum.url,
                title: trackinfo.title.clone(),
                duration: trackinfo.duration.map(Duration::from_secs_f64),
                created_time: tralbum
                    .current
                    .new_date
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc2822)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                published_time: tralbum
                    .current
                    .publish_date
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc2822)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                modified_time: tralbum
                    .current
                    .mod_date
                    .as_deref()
                    .map(chrono::DateTime::parse_from_rfc2822)
                    .map(Result::ok)
                    .flatten()
                    .map(chrono::DateTime::<Utc>::from),
                ..Default::default()
            },

            established_formats: trackinfo
                .file
                .keys()
                .into_iter()
                .map(|quality| MediaFormatEstablished {
                    url: MediaFormatURL::HTTP(
                        Url::parse(&trackinfo.file.get(quality).unwrap().to_string()).unwrap(),
                        HTTPDownloadOptions::default(),
                    ),
                    details: MediaFormatDetails {
                        id: quality.to_string(),
                        breed: FormatBreed::Audio,
                        audio_details: Some(AudioDetails {
                            ..Default::default()
                        }),
                        video_details: None,
                    },
                })
                .collect(),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use reytan_extractor_api::url::Url;
    use reytan_extractor_api::{
        ExtractLevel, Extractable, ExtractionContext, RecordingExtractor, URLMatcher,
    };

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
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny")
                    .unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .expect("extraction");
        let metadata = recording.metadata;
        assert_eq!(metadata.title, "Make that Skirt go Spinny");
        assert_eq!(recording.established_formats.len(), 1);
    }

    #[tokio::test]
    async fn do_fetch_album_track() {
        let bandcamp = BandcampRE {};
        let recording = bandcamp
            .extract_recording(
                &ExtractionContext::new().unwrap(),
                &Url::parse("https://penelopescott.bandcamp.com/track/r-t-2").unwrap(),
                &Extractable {
                    metadata: ExtractLevel::Basic,
                    playback: ExtractLevel::Extended,
                },
            )
            .await
            .expect("extraction");
        let metadata = recording.metadata;
        assert_eq!(metadata.title, "Rät");
        assert_eq!(recording.established_formats.len(), 1);
    }
}
