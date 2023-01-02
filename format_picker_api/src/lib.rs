pub use reytan_extractor_api::{
    anyhow, async_trait, Extraction, MediaFormatDetails, MediaMetadata, SubtitleDetails,
};
use reytan_extractor_api::{MediaFormatEstablished, SubtitlePointerURL};
use serde::{Deserialize, Serialize};

/// Helper enum for choosing the formats to be downloaded/merged/played.
/// String stands for the `id` field of the entity.
///
/// Oversimplified, but should suit typical use cases.
#[derive(Deserialize, Serialize, Debug)]
pub enum FormatSelection<T> {
    Full(T),
    ExtractVideo(T),
    ExtractAudio(T),
}

impl<T> FormatSelection<T> {
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> FormatSelection<U> {
        match self {
            FormatSelection::Full(x) => FormatSelection::Full(f(x)),
            FormatSelection::ExtractVideo(x) => FormatSelection::ExtractVideo(f(x)),
            FormatSelection::ExtractAudio(x) => FormatSelection::ExtractAudio(f(x)),
        }
    }
}

impl<T> FormatSelection<T> {
    pub fn map_ref<U, F: FnOnce(&T) -> U>(&self, f: F) -> FormatSelection<U> {
        match self {
            FormatSelection::Full(x) => FormatSelection::Full(f(x)),
            FormatSelection::ExtractVideo(x) => FormatSelection::ExtractVideo(f(x)),
            FormatSelection::ExtractAudio(x) => FormatSelection::ExtractAudio(f(x)),
        }
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
/// A helper type to be outputted by the format pickers.
/// For actual stuff to use for download, see [`DownloadList`].
pub struct DownloadSelection {
    pub formats: Option<Vec<FormatSelection<String>>>,
    pub subtitles: Option<Vec<String>>,
}

#[derive(Serialize, Debug)]
pub struct DownloadList<'a> {
    pub formats: Vec<FormatSelection<&'a MediaFormatEstablished>>,
    pub subtitles: Vec<&'a SubtitlePointerURL>,
}

impl<'a> DownloadList<'a> {
    pub fn from(
        selection: &DownloadSelection,
        extraction: &'a Extraction,
    ) -> anyhow::Result<DownloadList<'a>> {
        Ok(DownloadList {
            formats: selection
                .formats
                .as_ref()
                .map(|fss| {
                    fss.iter()
                        .map(|fs| {
                            fs.map_ref(|sid| {
                                extraction
                                    .established_formats
                                    .iter()
                                    .find(|ef| &ef.details.id == sid)
                                    .unwrap()
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            subtitles: selection
                .subtitles
                .as_ref()
                .map(|ses| {
                    ses.iter()
                        .map(|sid| {
                            extraction
                                .established_subtitles
                                .iter()
                                .find(|es| &es.details.lang == sid)
                                .unwrap()
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

#[async_trait]
pub trait FormatPicker {
    async fn pick_formats(
        &self,
        selector: &str,
        extraction: &Extraction,
    ) -> anyhow::Result<DownloadSelection>;
}
