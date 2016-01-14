extern crate rand;
extern crate time;
extern crate rustc_serialize;
extern crate eventual;
extern crate libcoinche;
extern crate iron;
extern crate url;
extern crate hyper;
extern crate bodyparser;

#[macro_use]
extern crate log;

// Small shortcuts for struct field en/de-coding
macro_rules! decode_field {
    ( $d:expr, $name:expr, $i:expr ) => {
        $d.read_struct_field($name, $i, |d| Decodable::decode(d))
    };
}

macro_rules! encode_field {
    ( $s:expr, $name:expr, $i:expr, $value:expr ) => {
        $s.emit_struct_field($name, $i, |s| $value.encode(s))
    };
}

mod event;
pub mod client;
pub mod server;

pub use event::*;

// Structures written by the server, read by the client

/// Player just joined a new party. He's given a player id, and his position.
#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub struct NewPartyInfo {
    /// Player ID, used in every request.
    pub player_id: u32,
    /// Player position in the table.
    pub player_pos: libcoinche::pos::PlayerPos,
}

#[derive(Clone,Debug,RustcEncodable,RustcDecodable)]
pub struct Error {
    pub error: String,
}


// Structures written by the client, read by the server.


#[derive(Clone,Debug,RustcDecodable,RustcEncodable)]
pub struct ContractBody {
    pub target: libcoinche::bid::Target,
    pub suit: libcoinche::cards::Suit,
}

#[derive(Clone,Debug,RustcDecodable,RustcEncodable)]
pub struct CardBody {
    pub card: libcoinche::cards::Card,
}
