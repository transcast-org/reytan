use crate::extractors::{api::{URLMatch, URLMatcher, RecordingExtractor, Extractable, Extraction, MediaMetadata, MediaPlayback, MediaFormat, FormatBreed, AudioDetails}, bandcamp::types::web_fragments::DataTralbum};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header;
use url::Url;
use nipper::Document;

pub struct BandcampRE {}

#[async_trait]
impl URLMatcher for BandcampRE {
    async fn match_extractor(self, url: &Url, _http: &reqwest::Client) -> Option<URLMatch> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        if let Some(hostname) = url.host_str() {
            let segments: Vec<&str> = url.path_segments().unwrap_or("".split('/')).collect();
            if !hostname.ends_with(".bandcamp.com") {
                let lkup = trust_dns_resolver::AsyncResolver
                    ::tokio_from_system_conf().unwrap()
                    .lookup(hostname, 
                        trust_dns_resolver::proto::rr::RecordType::CNAME, 
                        trust_dns_resolver::proto::xfer::DnsRequestOptions::default())
                        .await
                        .expect("dns response");
                let maybe_record = lkup.record_iter().last();
                match maybe_record {
                    None => return None,
                    Some(record) => if let Some(data) = record.data() {
                        if let Some(cname) = data.as_cname() {
                            if cname.to_string() != "dom.bandcamp.com." {
                                return None
                            }
                        } else {
                            return None
                        }
                    } else {
                        return None
                    }
                }
            }
            match segments.get(0).unwrap_or(&"") {
                &"track" => {
                    if let Some(id) = segments.get(1) {
                        return Some(URLMatch {
                            // making sure there's no extra things
                            id: format!("https://{}/track/{}", hostname, id),
                        });
                    }
                }
                _ => (),
            }
        }
        return None;
    }
}

#[async_trait]
impl RecordingExtractor for BandcampRE {
    async fn extract_recording(self,http: &reqwest::Client,id: &str, _wanted: &Extractable,) -> Result<Extraction> {
        let webpage = http.get(id).header(header::USER_AGENT, "Mozilla/5.0 (Linux x86_64; rv:102.0) Gecko/20100101 Firefox/102.0").send().await?.text().await?;
        let document = Document::from(&webpage);
        let dtralbum= document.select("script[data-tralbum]").attr("data-tralbum").unwrap().to_string();
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
                    trackinfo.file.keys().into_iter().map(|quality| MediaFormat {
                        id: quality.to_string(),
                        breed: FormatBreed::Audio,
                        url: trackinfo.file.get(quality).unwrap().to_string(),
                        audio_details: Some(AudioDetails {
                            ..Default::default()
                        }),
                        ..Default::default()
                    }).collect()
                },
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{build_http, extractors::api::{URLMatcher, RecordingExtractor, ExtractLevel}};
    use url::Url;

    use super::BandcampRE;

    #[tokio::test]
    async fn match_track_url() {
        let bandcamp = BandcampRE {};
        let mtch = bandcamp
            .match_extractor(
                &Url::parse("http://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny/?utm_source=your_mom#aoeu")
                    .unwrap(), 
                &build_http())
            .await
            .expect("valid match");
        assert_eq!(
            mtch.id,
            "https://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny"
        );
    }

    #[tokio::test]
    async fn match_embedded_domain_track_url() {
        let bandcamp = BandcampRE {};
        let mtch = bandcamp
            .match_extractor(
                &Url::parse("https://music.gabakulka.com/track/alright-amanda")
                    .unwrap(), 
                &build_http())
            .await
            .expect("valid match");
        assert_eq!(
            mtch.id,
            "https://music.gabakulka.com/track/alright-amanda"
        );
    }

    #[tokio::test]
    async fn do_fetch_tralbum() {
        let bandcamp = BandcampRE {};
        let recording = bandcamp
        .extract_recording(&build_http(), "https://miraonthewall.bandcamp.com/track/make-that-skirt-go-spinny", &crate::extractors::api::Extractable { metadata: ExtractLevel::Basic, playback: ExtractLevel::Extended }).await.expect("extraction");
        let metadata = recording.metadata.expect("metadata");
        assert_eq!(metadata.title, "Make that Skirt go Spinny");
        let playback = recording.playback.expect("playback");
        assert_eq!(playback.formats.len(), 1);
    }

    #[tokio::test]
    async fn do_fetch_album_track() {
        let bandcamp = BandcampRE {};
        let recording = bandcamp
        .extract_recording(&build_http(), "https://penelopescott.bandcamp.com/track/r-t-2", &crate::extractors::api::Extractable { metadata: ExtractLevel::Basic, playback: ExtractLevel::Extended }).await.expect("extraction");
        let metadata = recording.metadata.expect("metadata");
        assert_eq!(metadata.title, "Rät");
        let playback = recording.playback.expect("playback");
        assert_eq!(playback.formats.len(), 1);
    }
}
