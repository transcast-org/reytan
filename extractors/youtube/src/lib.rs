#[macro_use]
extern crate smart_default;

mod common;
pub mod recording;
pub mod tab;
pub mod types;

use once_cell::sync::Lazy;
pub use recording::YoutubeRE;
use reytan_extractor_api::{AnyExtractor, NewExtractor};
pub use tab::YoutubeTabLE;

pub static EXTRACTORS: Lazy<Vec<AnyExtractor>> = Lazy::new(|| {
    vec![
        AnyExtractor::List(Box::new(YoutubeTabLE::new())),
        AnyExtractor::Recording(Box::new(YoutubeRE::new())),
    ]
});
