use anyhow::Result;
use clap::Parser;
use reytan::{AnyExtraction, CoreClient, ExtractLevel, Extractable, MediaFormatURL};
use url::Url;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg()]
    url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = CoreClient::new();

    println!("extracting {}", &args.url);
    let extraction = client
        .extract_url(
            &Url::parse(&args.url)?,
            &Extractable {
                metadata: ExtractLevel::Extended,
                playback: ExtractLevel::Extended,
            },
        )
        .await?
        .unwrap();

    match extraction {
        AnyExtraction::Recording(e) => {
            let meta = e.metadata.unwrap();
            println!("{:#?}", meta);
            let play = e.playback.unwrap();
            for fmt in &play.formats {
                let fmt_url = fmt.url.get().await?;
                let printable_url = match fmt_url {
                    MediaFormatURL::HTTP(u) => format!("[HTTP] {}", u),
                    MediaFormatURL::HLS(u) => format!("[HLS] {}", u),
                };
                println!("{}: {}", &fmt.id, &printable_url);
            }
        }
        AnyExtraction::List(_) => todo!(),
    }

    Ok(())
}
