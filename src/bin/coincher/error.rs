use hyper;
use url;
use rustc_serialize::json;

#[derive(Debug)]
pub enum Error {
    Url(url::ParseError),
    Hyper(hyper::Error),
    Json(json::DecoderError),
}

impl From<url::ParseError> for Error {
    fn from(err: url::ParseError) -> Self {
        Error::Url(err)
    }
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Hyper(err)
    }
}

impl From<json::ParserError> for Error {
    fn from(err: json::ParserError) -> Self {
        Error::Json(json::DecoderError::ParseError(err))
    }
}

impl From<json::DecoderError> for Error {
    fn from(err: json::DecoderError) -> Self {
        Error::Json(err)
    }
}


