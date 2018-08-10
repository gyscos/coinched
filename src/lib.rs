use serde_derive::{Deserialize, Serialize};

pub mod client;
mod event;
pub mod server;

pub use event::*;

// Structures written by the server, read by the client

/// Player just joined a new party. He's given a player id, and his position.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewPartyInfo {
    /// Player ID, used in every request.
    pub player_id: u32,
    /// Player position in the table.
    pub player_pos: libcoinche::pos::PlayerPos,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error {
    pub error: String,
}

// Structures written by the client, read by the server.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractBody {
    pub target: libcoinche::bid::Target,
    pub suit: libcoinche::cards::Suit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CardBody {
    pub card: libcoinche::cards::Card,
}
