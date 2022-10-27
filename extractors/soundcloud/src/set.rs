use itertools::intersperse;
use qstring::QString;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::url::Url;
use reytan_extractor_api::{
    async_trait, AnyExtraction, Extraction, ExtractionContext, ListBreed, ListContinuation,
    ListExtraction, ListExtractor, NewExtractor, URLMatcher,
};

use crate::common::get_api_request;
use crate::types::{MaybeTrackInfo, Set, Track};

pub struct SoundcloudSetLE {}

impl NewExtractor for SoundcloudSetLE {
    fn new() -> Self {
        SoundcloudSetLE {}
    }
}

impl URLMatcher for SoundcloudSetLE {
    fn match_extractor(&self, url: &Url) -> bool {
        let segments: Vec<_> = url.path_segments().unwrap().collect();
        Some(url)
            .filter(|_| segments.get(1) == Some(&"sets"))
            .is_some()
    }
}

#[async_trait]
impl ListExtractor for SoundcloudSetLE {
    async fn extract_list_initial(
        &self,
        ctx: &ExtractionContext,
        url: &Url,
    ) -> Result<ListExtraction> {
        let set: Set = get_api_request(
            ctx,
            "/resolve",
            &mut QString::new(vec![("url", url.as_str())]),
        )
        .await?;
        Ok(ListExtraction {
            id: set.id.to_string(),
            breed: ListBreed::Album,
            title: set.title,
            is_endless: false,
            entries: Some(Ok(set
                .tracks
                .iter()
                .flat_map(MaybeTrackInfo::track)
                .map(Extraction::from)
                .map(AnyExtraction::Recording)
                .collect())),
            continuation: Some(
                intersperse(
                    set.tracks
                        .iter()
                        .flat_map(MaybeTrackInfo::stub)
                        .map(|s| s.id.to_string()),
                    ",".to_string(),
                )
                .collect(),
            ),
        })
    }

    async fn extract_list_continuation(
        &self,
        ctx: &ExtractionContext,
        id: &str,
        continuation: &str,
    ) -> Result<ListContinuation> {
        let track_ids = continuation.split(",");
        let tracks: Vec<Track> = get_api_request(
            ctx,
            "/tracks",
            &mut QString::new(vec![(
                "ids",
                intersperse(track_ids.clone().take(50), ",").collect::<String>(),
            )]),
        )
        .await?;
        Ok(ListContinuation {
            id: id.to_string(),
            entries: Some(Ok(tracks
                .into_iter()
                .map(Extraction::from)
                .map(AnyExtraction::Recording)
                .collect())),
            continuation: Some(intersperse(track_ids.skip(50), ",").collect()),
        })
    }
}

#[cfg(test)]
mod tests {
    use reytan_extractor_api::{url::Url, ExtractionContext, ListBreed, ListExtractor, URLMatcher};

    use crate::set::SoundcloudSetLE;

    #[tokio::test]
    async fn test_extraction_basic() {
        let ctx = ExtractionContext::new();
        let extractor = SoundcloudSetLE {};
        let url = &Url::parse("https://soundcloud.com/goophouse/sets/goop-house-volume-7").unwrap();
        let mtch = extractor.match_extractor(url);
        assert_eq!(mtch, true);

        let initial = extractor.extract_list_initial(&ctx, &url).await.unwrap();
        assert_eq!(initial.id, "1459801735");
        assert_eq!(initial.breed, ListBreed::Album);
        assert_eq!(initial.is_endless, false);
        assert_eq!(initial.entries.as_ref().unwrap().as_ref().unwrap().len(), 5);
        assert!(initial.continuation.is_some());

        let continued = extractor
            .extract_list_continuation(&ctx, &initial.id, &initial.continuation.unwrap())
            .await
            .unwrap();
        assert!(continued.continuation.is_some());
        assert_eq!(continued.entries.unwrap().unwrap().len(), 50);
    }
}
