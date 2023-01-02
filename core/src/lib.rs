use once_cell::sync::Lazy;
use reytan_extractor_api::anyhow::Result;
use reytan_extractor_api::url::Url;
pub use reytan_extractor_api::*;
use reytan_format_picker_api::FormatPicker;
pub use reytan_format_picker_api::{DownloadList, FormatSelection};
#[cfg(feature = "jrsonnet")]
use reytan_format_picker_jrsonnet::JrsonnetFormatPicker;

pub static DEFAULT_EXTRACTOR_LIST: Lazy<Vec<&AnyExtractor>> = Lazy::new(|| {
    let l = vec![].into_iter();

    #[cfg(feature = "bandcamp")]
    let l = l.chain(reytan_extractor_bandcamp::EXTRACTORS.iter());

    #[cfg(feature = "soundcloud")]
    let l = l.chain(reytan_extractor_soundcloud::EXTRACTORS.iter());

    #[cfg(feature = "youtube")]
    let l = l.chain(reytan_extractor_youtube::EXTRACTORS.iter());

    l.collect()
});

pub struct CoreClient<'a> {
    extractors: Vec<&'a AnyExtractor>,
    context: ExtractionContext,
    format_picker: Box<dyn FormatPicker>,
}

impl<'a> CoreClient<'a> {
    pub fn new() -> Self {
        CoreClient {
            extractors: DEFAULT_EXTRACTOR_LIST.to_vec(),
            context: ExtractionContext::new(),
            #[cfg(feature = "jrsonnet")]
            format_picker: Box::new(JrsonnetFormatPicker::new()),
        }
    }

    pub async fn extract_url(
        &self,
        url: &Url,
        wanted: &Extractable,
    ) -> Result<Option<AnyExtraction>> {
        for extractor in &self.extractors {
            if extractor.match_extractor(url) {
                return extractor
                    .extract_info(&self.context, url, wanted)
                    .await
                    .map(Option::Some);
            }
        }
        Ok(None)
    }

    pub async fn pick_formats(
        &self,
        selector: &str,
        extraction: &'a Extraction,
    ) -> Result<DownloadList> {
        DownloadList::from(
            &self
                .format_picker
                .pick_formats(selector, extraction)
                .await?,
            extraction,
        )
    }
}
