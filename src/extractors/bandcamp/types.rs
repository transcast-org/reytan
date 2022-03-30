pub mod web_fragments {
    use serde::Deserialize;

    #[derive(SmartDefault, Deserialize, PartialEq, Debug)]
    pub struct DataTralbum {
        pub current: parts::Current,
        pub trackinfo: Vec<parts::Trackinfo>,
        pub url: String,
    }

    pub mod parts {
        use std::collections::HashMap;

        use serde::Deserialize;

        #[derive(SmartDefault, Deserialize, PartialEq, Debug)]
        pub struct Trackinfo {
            /// file formats ("mp3-128") resolved to the HTTPS url
            pub file: HashMap<String, String>,
            /// URL to the track, relative
            pub title_link: String,
            pub title: String,
            pub lyrics: Option<String>,
            pub duration: Option<f64>,
        }

        #[derive(SmartDefault, Deserialize, PartialEq, Debug)]
        pub struct Current {
            pub title: String,
        }
    }
}
