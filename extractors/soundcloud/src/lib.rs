mod common;
pub mod set;
pub mod track;
mod types;

use once_cell::sync::Lazy;
use reytan_extractor_api::{AnyExtractor, NewExtractor};
pub use set::SoundcloudSetLE;
pub use track::SoundcloudRE;

pub static EXTRACTORS: Lazy<Vec<AnyExtractor>> = Lazy::new(|| {
    vec![
        AnyExtractor::List(Box::new(SoundcloudSetLE::new())),
        AnyExtractor::Recording(Box::new(SoundcloudRE::new())),
    ]
});
