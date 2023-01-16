use http::uri::Uri;
use url::{Position, Url};

pub enum AnyFuckingURL {
    Url(Url),
    Uri(Uri),
}

impl AnyFuckingURL {
    pub fn to_uri(self) -> Uri {
        match self {
            AnyFuckingURL::Url(url) => {
                let mut builder = Uri::builder();
                builder = builder.scheme(url.scheme());
                builder = builder.authority(&url[Position::BeforeUsername..Position::AfterPort]);
                builder = builder.path_and_query(&url[Position::BeforePath..Position::AfterQuery]);
                builder.build().unwrap()
            }
            AnyFuckingURL::Uri(uri) => uri,
        }
    }
}

impl From<Uri> for AnyFuckingURL {
    fn from(value: Uri) -> Self {
        AnyFuckingURL::Uri(value)
    }
}

impl From<Url> for AnyFuckingURL {
    fn from(value: Url) -> Self {
        AnyFuckingURL::Url(value)
    }
}

pub fn uri<U>(anyfuckingurl: U) -> Uri
where
    U: Into<AnyFuckingURL>,
{
    anyfuckingurl.into().to_uri()
}
