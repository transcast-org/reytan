use std::env::current_dir;
use std::path::Path;

use once_cell::sync::Lazy;
use reytan_download::Downloader;
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
    downloader: Downloader,
}

impl<'a> CoreClient<'a> {
    pub fn new() -> Self {
        CoreClient {
            extractors: DEFAULT_EXTRACTOR_LIST.to_vec(),
            context: ExtractionContext::new().unwrap(),
            #[cfg(feature = "jrsonnet")]
            format_picker: Box::new(JrsonnetFormatPicker::new()),
            downloader: Downloader::new(),
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

    pub async fn download(&self, url: &Url, wanted: &Extractable, selector: &str) -> Result<()> {
        let extraction = self
            .extract_url(url, wanted)
            .await?
            .expect("nothing got extracted");
        match extraction {
            AnyExtraction::Recording(recording) => {
                self.download_recording(&recording, selector).await
            }
            _ => todo!(),
        }
    }

    pub async fn download_from_list<P>(
        &self,
        download_list: &DownloadList<'a>,
        output: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        self.downloader
            .download_from_list(&self.context, download_list, output)
            .await
    }

    async fn download_recording(&self, extraction: &'a Extraction, selector: &str) -> Result<()> {
        let download_list = self.pick_formats(selector, extraction).await?;
        self.downloader
            .download_from_list(
                &self.context,
                &download_list,
                current_dir()?.join(&extraction.metadata.id),
            )
            .await
    }
}
