use reytan_download_http::HTTPDownloader;
use reytan_download_types::anyhow::Result;
use reytan_download_types::{DownloadList, ExtractionContext, FormatSelection, MediaFormatURL};
use std::path::Path;

pub struct Downloader {
    http: HTTPDownloader,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            http: HTTPDownloader::new(),
        }
    }

    pub async fn download_from_list<'a, P>(
        &self,
        ctx: &ExtractionContext,
        download_list: &'a DownloadList<'a>,
        output_: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let output = output_.as_ref();
        let filenames_and_formats =
            download_list
                .formats
                .iter()
                .map(|format_sel| match format_sel {
                    &FormatSelection::Full(format)
                    | &FormatSelection::ExtractVideo(format)
                    | &FormatSelection::ExtractAudio(format) => (
                        output.with_file_name(format!(
                            "{}.f{}.{}",
                            output.file_stem().unwrap().to_str().unwrap(),
                            format.details.id,
                            output.extension().unwrap().to_str().unwrap(),
                        )),
                        format_sel,
                    ),
                });

        for (format_output, format_sel) in filenames_and_formats {
            match format_sel {
                &FormatSelection::Full(format)
                | &FormatSelection::ExtractVideo(format)
                | &FormatSelection::ExtractAudio(format) => {
                    self.download_format(ctx, &format.url, &format_output)
                        .await?;
                }
            }
        }

        Ok(())
    }

    pub async fn download_format<'a, P>(
        &self,
        ctx: &ExtractionContext,
        format: &'a MediaFormatURL,
        output: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        match &format {
            &MediaFormatURL::HTTP(url, options) => {
                self.http.download_format(ctx, url, options, output).await?;
                Ok(())
            }
            _ => todo!(),
        }
    }
}
