use crate::{CardBody, ContractBody, EventType};
use libcoinche::{bid, cards, pos};

mod client;
// pub mod http;

pub use self::client::{run, Client};

#[derive(Serialize, Deserialize)]
pub enum AuctionAction {
    Leave,
    Pass,
    Coinche,
    Bid {
        trump: cards::Suit,
        target: bid::Target,
    },
}

#[derive(Serialize, Deserialize)]
pub enum GameAction {
    Leave,
    PlayCard { card: cards::Card },
}

/// The frontend is the part of the client that interacts with the player.
///
/// The backend will
pub trait Frontend {
    type B: Backend;

    fn show_error(&mut self, error: B::Error);
    fn unexpected_event(&mut self, event: EventType);
    fn party_cancelled(&mut self, msg: &str);

    fn show_card_played(&mut self, pos: pos::PlayerPos, card: cards::Card);
    fn show_trick_over(&mut self, winner: pos::PlayerPos);
    fn ask_card(&mut self) -> GameAction;
    fn ask_bid(&mut self) -> AuctionAction;
    fn game_over(&mut self, points: [i32; 2], winner: pos::Team, scores: [i32; 2]);

    fn show_pass(&mut self, pos: pos::PlayerPos);
    fn show_coinche(&mut self, pos: pos::PlayerPos);
    fn show_bid(&mut self, pos: pos::PlayerPos, suit: cards::Suit, target: bid::Target);

    /// Auction cancelled, back to the start.
    fn auction_cancelled(&mut self);
    /// Auction is complete, we can play now!
    fn auction_over(&mut self, contract: &bid::Contract);

    fn start_game(&mut self, first: pos::PlayerPos, hand: cards::Hand);
}

/// The backend is the part of the client that interacts with the server.
pub trait Backend {
    type Error;

    /// Wait for the next event and return it.
    fn wait(&mut self) -> Result<EventType, Self::Error>;

    /// Make a bid offer.
    ///
    /// Return the event caused by the action.
    fn bid(&mut self, contract: ContractBody) -> Result<EventType, Self::Error>;

    /// Pass during auction.
    ///
    /// Return the event caused by the action.
    fn pass(&mut self) -> Result<EventType, Self::Error>;

    fn coinche(&mut self) -> Result<EventType, Self::Error>;

    fn play_card(&mut self, card: CardBody) -> Result<EventType, Self::Error>;
}
