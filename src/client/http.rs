use std::io;
use rustc_serialize::Decodable;
use rustc_serialize::json;
use hyper::client::IntoUrl;
use std::io::Read;
use libcoinche::pos;
use hyper::header::ContentType;
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use url;
use hyper;

use {NewPartyInfo, Event, EventType, ContractBody, CardBody};

use super::Backend;

/// HTTP coinched client.
///
/// Provides an abstraction over HTTP requests.
pub struct HttpBackend {
    player_id: u32,
    pub pos: pos::PlayerPos,

    event_id: usize,

    host: String, /* It used to include a re-usable hyper::Client,
                   * but it would lead to failed request if too
                   * long happened between two queries. */
}

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

/// Helper method to decode a `T: Decodable` from a reader.
///
/// (`json::decode` only works from a string)
fn from_reader<R: Read, T: Decodable>(r: &mut R) -> Result<T, Error> {
    let json = try!(json::Json::from_reader(r));
    // println!("Json: {:?}", json);
    let mut decoder = json::Decoder::new(json);
    let result = try!(Decodable::decode(&mut decoder));
    Ok(result)
}

/// Leave the party on drop.
/// TODO: handle "soft" exit with reconnection?
impl Drop for HttpBackend {
    fn drop(&mut self) {
        let leave_url = format!("http://{}/leave/{}", self.host, self.player_id);
        hyper::Client::new().post(&leave_url).send().unwrap();
    }
}

impl HttpBackend {
    /// Creates a client to connect to the given server, once logged in.
    fn new(host: &str, player_id: u32, pos: pos::PlayerPos) -> Self {

        HttpBackend {
            player_id: player_id,
            pos: pos,
            event_id: 0,
            host: host.to_string(),
        }
    }

    /// Attempt to join a game on the given host.
    pub fn join(host: &str) -> Result<Self, Error> {
        let client = hyper::Client::new();

        let join_url = try!(format!("http://{}/join", host).into_url());
        println!("Connecting to {}", host);
        let mut response = try!(client.post(join_url).send());
        let party: NewPartyInfo = try!(from_reader(&mut response));

        Ok(HttpBackend::new(host, party.player_id, party.player_pos))
    }

    /// Parse and return an event from the given reader.
    fn read_event<R: io::Read>(&mut self, r: &mut R) -> Result<EventType, Error> {
        let event: Event = try!(from_reader(r));

        self.event_id = event.id + 1;

        Ok(event.event)
    }
}

impl Backend for HttpBackend {
    type Error = Error;

    fn wait(&mut self) -> Result<EventType, Error> {
        let wait_url = format!("http://{}/wait/{}/{}",
                               &self.host,
                               self.player_id,
                               self.event_id);
        let mut response = try!(hyper::Client::new().get(&wait_url).send());
        self.read_event(&mut response)
    }

    fn bid(&mut self, contract: ContractBody) -> Result<EventType, Error> {
        let bid_url = format!("http://{}/bid/{}", self.host, self.player_id);
        let body = json::encode(&contract).unwrap();
        let mut response = try!(hyper::Client::new()
                                    .post(&bid_url)
                                    .header(ContentType(Mime(TopLevel::Application,
                                                             SubLevel::Json,
                                                             vec![(Attr::Charset, Value::Utf8)])))
                                    .body(&body)
                                    .send());
        self.read_event(&mut response)
    }

    fn pass(&mut self) -> Result<EventType, Error> {
        let pass_url = format!("http://{}/pass/{}", self.host, self.player_id);
        let mut response = try!(hyper::Client::new().post(&pass_url).send());
        self.read_event(&mut response)
    }

    fn coinche(&mut self) -> Result<EventType, Error> {
        let coinche_url = format!("http://{}/coinche/{}", self.host, self.player_id);
        let mut response = try!(hyper::Client::new().post(&coinche_url).send());
        self.read_event(&mut response)
    }

    fn play_card(&mut self, card: CardBody) -> Result<EventType, Error> {
        let play_url = format!("http://{}/play/{}", self.host, self.player_id);
        let body = json::encode(&card).unwrap();
        let mut response = try!(hyper::Client::new()
                                    .post(&play_url)
                                    .header(ContentType(Mime(TopLevel::Application,
                                                             SubLevel::Json,
                                                             vec![(Attr::Charset, Value::Utf8)])))
                                    .body(&body)
                                    .send());
        self.read_event(&mut response)
    }
}
