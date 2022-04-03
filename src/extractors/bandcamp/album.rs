use crate::extractors::{
    api::{
        AnyExtraction, Extraction, ListBreed, ListContinuation, ListExtraction, ListExtractor,
        MediaMetadata, MediaPlayback, URLMatch, URLMatcher,
    },
    bandcamp::types::web_fragments::DataTralbum,
};
use anyhow::Result;
use async_trait::async_trait;
use nipper::Document;
use reqwest::header;
use url::Url;

pub struct BandcampAlbumLE {}

#[async_trait]
impl URLMatcher for BandcampAlbumLE {
    async fn match_extractor(self, url: &Url, _http: &reqwest::Client) -> Option<URLMatch> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return None;
        }
        if let Some(hostname) = url.host_str() {
            let segments: Vec<&str> = url.path_segments().unwrap_or("".split('/')).collect();
            if hostname.ends_with(".bandcamp.com") {
                match segments.get(0).unwrap_or(&"") {
                    &"album" => {
                        if let Some(id) = segments.get(1) {
                            return Some(URLMatch {
                                // making sure there's no extra things
                                id: format!("https://{}/album/{}", hostname, id),
                            });
                        }
                    }
                    _ => (),
                }
            }
        }
        return None;
    }
}

#[async_trait]
impl ListExtractor for BandcampAlbumLE {
    async fn extract_list_initial(
        self,
        http: &reqwest::Client,
        id: &str,
    ) -> Result<ListExtraction> {
        let webpage = http
            .get(id)
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
        Ok(ListExtraction {
            id: tralbum.url,
            breed: ListBreed::Album,
            title: tralbum.current.title,
            is_endless: false,
            entries: {
                Some(Ok(tralbum
                    .trackinfo
                    .into_iter()
                    .map(|ti| {
                        AnyExtraction::Recording(Extraction {
                            metadata: Some(MediaMetadata {
                                id: ti.title_link,
                                title: ti.title,
                                ..Default::default()
                            }),
                            playback: Some(MediaPlayback {
                                formats: vec![],
                                ..Default::default()
                            }),
                            ..Default::default()
                        })
                    })
                    .collect()))
            },
            ..Default::default()
        })
    }

    async fn extract_list_continuation(
        self,
        _http: &reqwest::Client,
        _id: &str,
        _continuation: &str,
    ) -> Result<ListContinuation> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        build_http,
        extractors::api::{ListBreed, ListExtractor, URLMatcher},
    };
    use url::Url;

    use super::BandcampAlbumLE;

    #[tokio::test]
    async fn match_album_url() {
        let bandcamp = BandcampAlbumLE {};
        let mtch = bandcamp
            .match_extractor(
                &Url::parse(
                    "http://miraonthewall.bandcamp.com/album/restoration/?utm_source=your_mom#aoeu",
                )
                .unwrap(),
                &build_http(),
            )
            .await
            .expect("valid match");
        assert_eq!(
            mtch.id,
            "https://miraonthewall.bandcamp.com/album/restoration"
        );
    }

    #[tokio::test]
    async fn do_fetch_full_album() {
        let bandcamp = BandcampAlbumLE {};
        let album = bandcamp
            .extract_list_initial(
                &build_http(),
                "https://penelopescott.bandcamp.com/album/public-void",
            )
            .await
            .expect("extraction");
        assert_eq!(album.title, "Public Void");
        assert_eq!(album.breed, ListBreed::Album);
        assert_eq!(album.continuation, None);
        assert_eq!(album.is_endless, false);
    }
}
