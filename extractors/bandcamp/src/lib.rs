#[macro_use]
extern crate smart_default;

pub mod album;
mod common;
pub mod track;
mod types;

pub use album::BandcampAlbumLE;
use once_cell::sync::Lazy;
use reytan_extractor_api::{AnyExtractor, NewExtractor};
pub use track::BandcampRE;

pub static EXTRACTORS: Lazy<Vec<AnyExtractor>> = Lazy::new(|| {
    vec![
        AnyExtractor::List(Box::new(BandcampAlbumLE::new())),
        AnyExtractor::Recording(Box::new(BandcampRE::new())),
    ]
});
