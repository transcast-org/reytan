use std::path::PathBuf;

use jrsonnet_evaluator::{EvaluationState, Val};
use reytan_format_picker_api::{anyhow, async_trait, DownloadSelection, Extraction, FormatPicker};
use serde_json::json;

pub struct JrsonnetFormatPicker {}

impl JrsonnetFormatPicker {
    pub fn new() -> Self {
        JrsonnetFormatPicker {}
    }
}

#[async_trait]
impl FormatPicker for JrsonnetFormatPicker {
    async fn pick_formats(
        &self,
        selector: &str,
        extraction: &Extraction,
    ) -> anyhow::Result<DownloadSelection> {
        self.pick_formats_real(selector, extraction)
    }
}
impl JrsonnetFormatPicker {
    fn pick_formats_real(
        &self,
        selector: &str,
        extraction: &Extraction,
    ) -> anyhow::Result<DownloadSelection> {
        let vm = EvaluationState::default();
        let input = json!({
            "media": &extraction.metadata,
            "formats": extraction.format_details(),
            "subtitles": extraction.subtitle_details(),
        });
        vm.add_tla("input".into(), Val::from(&input));
        Ok(serde_json::from_str(
            &vm.with_stdlib()
                .evaluate_snippet_raw(PathBuf::from("jsonnet_format").into(), selector.into())
                .and_then(|v| vm.with_tla(v))
                .and_then(|v| vm.manifest(v))
                .map_err(|e| anyhow::Error::msg(e.error().to_string()))?
                .to_string(),
        )?)
    }
}
