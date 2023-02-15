use std::env::current_dir;

use anyhow::Result;
use clap::Parser;
use reytan::{
    AnyExtraction, CoreClient, ExtractLevel, Extractable, Extraction, FormatSelection,
    MediaFormatURL,
};
use url::Url;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg()]
    url: String,

    #[arg(long)]
    jsonnet_format: Option<String>,
}

struct Reyt<'a> {
    args: Args,
    client: CoreClient<'a>,
}

impl<'a> Reyt<'a> {
    fn new() -> Self {
        Self {
            args: Args::parse(),
            client: CoreClient::new(),
        }
    }

    async fn main(&self) -> Result<()> {
        println!("extracting {}", &self.args.url);
        let extraction = self
            .client
            .extract_url(
                &Url::parse(&self.args.url)?,
                &Extractable {
                    metadata: ExtractLevel::Extended,
                    playback: ExtractLevel::Extended,
                },
            )
            .await?
            .unwrap();

        match extraction {
            AnyExtraction::Recording(e) => {
                self.handle_extraction(&e).await?;
            }
            AnyExtraction::List(_) => todo!(),
        }

        Ok(())
    }

    async fn handle_extraction(&self, e: &Extraction) -> Result<()> {
        // println!("{:#?}", e.metadata);
        println!("id: {}\ntitle: {}", e.metadata.id, e.metadata.title);
        let download_selection = if let Some(selector) = &self.args.jsonnet_format {
            self.client.pick_formats(selector, e).await?
        } else {
            todo!()
        };
        // println!("{:#?}", download_selection);
        println!("download selection:");
        for (i, format) in download_selection.formats.iter().enumerate() {
            match format {
                FormatSelection::Full(f) => {
                    println!(
                        "{i}: full: [{}] {}",
                        f.details.id,
                        printable_format_url(&f.url)
                    )
                }
                FormatSelection::ExtractVideo(f) => {
                    println!(
                        "{i}: extract video: [{}] {}",
                        f.details.id,
                        printable_format_url(&f.url)
                    )
                }
                FormatSelection::ExtractAudio(f) => {
                    println!(
                        "{i}: extract audio: [{}] {}",
                        f.details.id,
                        printable_format_url(&f.url)
                    )
                }
            }
        }
        println!("performing downloads");
        self.client
            .download_from_list(
                &download_selection,
                current_dir()?.join(format!("{}.{}", e.metadata.id, "mp4")),
            )
            .await?;
        Ok(())
    }
}
fn printable_format_url(mfu: &MediaFormatURL) -> String {
    match mfu {
        MediaFormatURL::HTTP(u, _) => format!("HTTP {}", u.as_str()),
        MediaFormatURL::HLS(u, _) => format!("HLS {}", u.as_str()),
        MediaFormatURL::DASH(u, _) => format!("DASH {}", u.as_str()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    Reyt::new().main().await
}
