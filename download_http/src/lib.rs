use reytan_download_types::anyhow::bail;
use reytan_download_types::anyhow::Result;
use reytan_download_types::ratmom::http::header;
use reytan_download_types::ratmom::prelude::*;
use reytan_download_types::ratmom::Request;
use reytan_download_types::{uri, ExtractionContext, HTTPDownloadOptions, MediaFormatURL, Url};
use std::path::Path;
use std::process::Command;

pub struct HTTPDownloader {}

impl HTTPDownloader {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn download_format<'a, P>(
        &self,
        _ctx: &ExtractionContext,
        url: &Url,
        options: &HTTPDownloadOptions,
        output: P,
    ) -> Result<()>
    where
        P: AsRef<Path>,
    {
        // TODO: make an actual downloader

        let mut cmd = Command::new("aria2c");
        cmd.args([
            "-c",
            "--console-log-level=warn",
            "--summary-interval=0",
            "--download-result=hide",
            "--http-accept-gzip=true",
            "--file-allocation=none",
            "-x16",
            "-j16",
            "-s16",
            "--min-split-size=1M",
        ]);

        cmd.arg("-o");
        cmd.arg(output.as_ref().as_os_str());

        if let Some(ua) = &options.connection.user_agent {
            cmd.arg("--header");
            cmd.arg(format!("user-agent: {}", ua));
        }

        cmd.arg("--");
        cmd.arg(url.as_str());

        let status = cmd.status()?;
        if !status.success() {
            bail!("aria2c exited with status {:?}", status.code());
        }

        Ok(())
    }
}
