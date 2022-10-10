use anyhow::Result;
use async_trait::async_trait;
use nipper::Document;
use reytan_context::reqwest::{self, header};
use reytan_extractor_api::{
    AnyExtraction, Extraction, ListBreed, ListContinuation, ListExtraction, ListExtractor,
    MediaMetadata, MediaPlayback, URLMatcher,
};
use url::Url;

use super::common::{_is_bandcamp, _path_is};
use super::types::web_fragments::DataTralbum;

pub struct BandcampAlbumLE {}

impl URLMatcher for BandcampAlbumLE {
    fn match_extractor(&self, url: &Url) -> bool {
        Some(url)
            .filter(|u| match u.scheme() {
                "http" | "https" => true,
                _ => false,
            })
            // url path starts with `/album/`
            .filter(|u| _path_is(u, "album"))
            .filter(_is_bandcamp)
            .is_some()
    }
}

#[async_trait]
impl ListExtractor for BandcampAlbumLE {
    async fn extract_list_initial(
        &self,
        http: &reqwest::Client,
        url: &Url,
    ) -> Result<ListExtraction> {
        let webpage = http
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
        &self,
        _http: &reqwest::Client,
        _id: &str,
        _continuation: &str,
    ) -> Result<ListContinuation> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use reytan_context::build_http;
    use reytan_extractor_api::{ListBreed, ListExtractor, URLMatcher};
    use url::Url;

    use super::BandcampAlbumLE;

    #[test]
    fn match_album_url() {
        let bandcamp = BandcampAlbumLE {};
        let mtch = bandcamp.match_extractor(
            &Url::parse(
                "http://miraonthewall.bandcamp.com/album/restoration/?utm_source=your_mom#aoeu",
            )
            .unwrap(),
        );
        assert_eq!(mtch, true);
    }

    #[tokio::test]
    async fn do_fetch_full_album() {
        let bandcamp = BandcampAlbumLE {};
        let album = bandcamp
            .extract_list_initial(
                &build_http(),
                &Url::parse("https://penelopescott.bandcamp.com/album/public-void").unwrap(),
            )
            .await
            .expect("extraction");
        assert_eq!(album.title, "Public Void");
        assert_eq!(album.breed, ListBreed::Album);
        assert_eq!(album.continuation, None);
        assert_eq!(album.is_endless, false);
    }
}
