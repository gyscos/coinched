extern crate rand;
extern crate time;
extern crate rustc_serialize;
extern crate eventual;
extern crate libcoinche;

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
mod game_manager;

pub use game_manager::*;
pub use event::*;

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
