extern crate coinched;
extern crate libcoinche;
extern crate hyper;
extern crate rustc_serialize;
extern crate url;

use std::io;
use std::io::{BufRead, Write};
use std::str::FromStr;
use rustc_serialize::Decodable;
use rustc_serialize::json;
use hyper::client::IntoUrl;
use std::convert::From;
use std::io::Read;
use libcoinche::pos::PlayerPos;
use hyper::header::{Header, ContentType};
use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};


// It used to include a re-usable hyper::Client,
// but it would lead to failed request if too
// long happened between two queries.
struct Client {
    player_id: u32,
    pos: PlayerPos,

    event_id: usize,

    host: String,
}

fn from_reader<R: Read, T: Decodable>(r: &mut R) -> Result<T, Error> {
    let json = try!(json::Json::from_reader(r));
    println!("Json: {:?}", json);
    let mut decoder = json::Decoder::new(json);
    let result = try!(Decodable::decode(&mut decoder));
    Ok(result)
}

impl Drop for Client {
    fn drop(&mut self) {
        println!("LEAVING");
        let leave_url = format!("http://{}/leave/{}", self.host, self.player_id);
        hyper::Client::new().post(&leave_url).send().unwrap();
    }
}

impl Client {
    fn new(host: &str, player_id: u32, pos: PlayerPos) -> Self {
        // Build the URL here

        Client {
            player_id: player_id,
            pos: pos,
            event_id: 0,
            host: host.to_string(),
        }
    }

    fn join(host: &str) -> Result<Self, Error> {
        let client = hyper::Client::new();

        let join_url = try!(format!("http://{}/join", host).into_url());
        let mut response = try!(client.post(join_url).send());
        let party: coinched::NewPartyInfo = try!(from_reader(&mut response));

        Ok(Client::new(host, party.player_id, party.player_pos))
    }

    fn read_event<R: io::Read>(&mut self, r: &mut R) -> Result<coinched::EventType, Error> {
        let event: coinched::Event = try!(from_reader(r));

        self.event_id = event.id + 1;

        Ok(event.event)
    }

    fn wait(&mut self) -> Result<coinched::EventType, Error> {
        let wait_url = format!("http://{}/wait/{}/{}",
                               &self.host,
                               self.player_id,
                               self.event_id);
        let mut response = try!(hyper::Client::new().get(&wait_url).send());
        self.read_event(&mut response)
    }

    fn bid(&mut self, contract: coinched::ContractBody) -> Result<coinched::EventType, Error> {
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

    fn pass(&mut self) -> Result<coinched::EventType, Error> {
        let pass_url = format!("http://{}/pass/{}", self.host, self.player_id);
        let mut response = try!(hyper::Client::new().post(&pass_url).send());
        self.read_event(&mut response)
    }

    fn coinche(&mut self) -> Result<coinched::EventType, Error> {
        let coinche_url = format!("http://{}/coinche/{}", self.host, self.player_id);
        let mut response = try!(hyper::Client::new().post(&coinche_url).send());
        self.read_event(&mut response)
    }
}

#[derive(Debug)]
enum Error {
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

fn play_game<F: FnMut() -> String>(client: &mut Client,
                                   first: PlayerPos,
                                   hand: libcoinche::cards::Hand,
                                   mut input: F)
                                   -> [i32; 2] {
    print!("Cards:\n[");
    for card in hand.list() {
        print!(" {}", card.to_string());
    }
    println!(" ]");

    let mut next = first;

    loop {
        let event = if next == client.pos {
            // Our turn
            print!("Bid? Write `pass` or `[80, 90, ... , Capot] [H,C,D,S]` or `coinche`.\n> ");
            io::stdout().flush().unwrap();
            let line = input();
            if &line == "pass" {
                client.pass()
            } else if &line == "coinche" {
                client.coinche()
            } else {
                let tokens: Vec<&str> = line.trim().split(" ").collect();
                if tokens.len() != 2 {
                    println!("Invalid number of tokens");
                    continue;
                }
                let target = match libcoinche::bid::Target::from_str(tokens[0]) {
                    Ok(target) => target,
                    _ => {
                        println!("Invalid target");
                        continue;
                    }
                };
                let suit = match libcoinche::cards::Suit::from_str(tokens[1]) {
                    Ok(suit) => suit,
                    Err(err) => {
                        println!("Invalid suit: {}", err);
                        continue;
                    }
                };

                let contract = coinched::ContractBody {
                    target: target,
                    suit: suit,
                };
                client.bid(contract)
            }
        } else {
            client.wait()
        };
        match event {
            Ok(coinched::EventType::FromPlayer(pos, e)) => {
                match e {
                    coinched::PlayerEvent::Bidded(trump, target) => {
                        println!("Player {:?} bidded {:?} on {:?}", pos, target, trump);
                        next = next.next();
                    }
                    coinched::PlayerEvent::Passed | coinched::PlayerEvent::Coinched => {
                        next = next.next()
                    }
                    _ => (),
                }
            }
            Ok(coinched::EventType::PartyCancelled(msg)) => panic!("party cancelled: {}", msg),
            Ok(_) => (),
            Err(err) => println!("Error: {:?}", err),
        }
    }
}

fn main() {
    // TODO: read this from arguments
    let host = "localhost:3000";

    // TODO: allow reconnecting to an existing game

    let mut client = Client::join(host).unwrap();
    let mut score: [i32; 2] = [0, 0];
    let mut stdin_reader = io::BufReader::new(io::stdin());
    let mut read_line = || {
        let mut buffer = String::new();
        stdin_reader.read_line(&mut buffer).unwrap();
        buffer.pop().unwrap();
        buffer
    };

    loop {
        let event = client.wait().unwrap();
        match event {
            coinched::EventType::NewGameRelative{first, hand} => {
                let new_scores = play_game(&mut client, first, hand, &mut read_line);
                score[0] += new_scores[0];
                score[1] += new_scores[1];
            }
            _ => panic!("Unexpected event: {:?}", event),
        }
    }

    // Then: card play
}
