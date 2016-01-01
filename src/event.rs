//! Event module

use rustc_serialize;
use libcoinche::{cards,bid,pos};

/// An event about a player.
#[derive(Clone)]
pub enum PlayerEvent {
    /// A player made a new bid in the auction.
    Bidded(cards::Suit, bid::Target),
    /// A player coinched the current bid in the auction.
    Coinched,
    /// A player passed in the auction.
    Passed,
    /// A player played a card.
    CardPlayed(cards::Card),
}

impl rustc_serialize::Encodable for PlayerEvent {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self {
            &PlayerEvent::Bidded(suit, target) => s.emit_struct("PlayerEvent", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "Bidded".encode(s)));
                try!(s.emit_struct_field("suit", 1, |s| suit.encode(s)));
                try!(s.emit_struct_field("target", 2, |s| target.encode(s)));
                Ok(())
            }),
            &PlayerEvent::Coinched => s.emit_struct("PlayerEvent", 1, |s| {
                s.emit_struct_field("type", 0, |s| "Coinched".encode(s))
            }),
            &PlayerEvent::Passed => s.emit_struct("PlayerEvent", 1, |s| {
                s.emit_struct_field("type", 0, |s| "Passed".encode(s))
            }),
            &PlayerEvent::CardPlayed(card) => s.emit_struct("PlayerEvent", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "CardPlayed".encode(s)));
                try!(s.emit_struct_field("card", 1, |s| card.encode(s)));
                Ok(())
            }),
        }
    }
}

/// Represents an event that can happen during the game.
#[derive(Clone)]
pub enum EventType {
    /// The party is cancelled. Contains an optional explanation.
    PartyCancelled(String),

    /// A player did something!
    FromPlayer(pos::PlayerPos, PlayerEvent),

    /// Bid over: contains the contract and the author
    BidOver(bid::Contract),
    /// The bid was cancelled, probably because no one bidded anything.
    /// A new game is probably on its way.
    BidCancelled,

    /// Trick over: contains the winner
    TrickOver { winner: pos::PlayerPos },

    /// New game: contains the first player, and the player's hand.
    /// For internal use only, it is never sent on the network.
    NewGame { first: pos::PlayerPos, hands: [cards::Hand;4] },
    /// New game event, translated for each player.
    NewGameRelative { first: pos::PlayerPos, hand: cards::Hand },

    /// Game over: contains scores
    GameOver { points: [i32;2], winner: pos::Team, scores: [i32;2] },
}

impl EventType {
    pub fn relativize(&self, from: pos::PlayerPos) -> Self {
        match self {
            &EventType::NewGame { first, hands } =>
                EventType::NewGameRelative { first: first, hand: hands[from.0] },
            _ => self.clone(),
        }
    }
}

// Ugly serialization...
impl rustc_serialize::Encodable for EventType {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self {
            &EventType::PartyCancelled(ref msg) => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "PartyCancelled".encode(s)));
                try!(s.emit_struct_field("msg", 1, |s| msg.encode(s)));
                Ok(())
            }),
            &EventType::BidCancelled => s.emit_struct("Event", 1, |s| {
                s.emit_struct_field("type", 0, |s| "BidCancelled".encode(s))
            }),
            &EventType::BidOver(ref contract) => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "BidOver".encode(s)));
                try!(s.emit_struct_field("contract", 1, |s| contract.encode(s)));
                Ok(())
            }),
            &EventType::TrickOver { winner: pos } => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "TrickOver".encode(s)));
                try!(s.emit_struct_field("pos", 1, |s| pos.encode(s)));
                Ok(())
            }),
            &EventType::FromPlayer(pos, ref event) => s.emit_struct("Event", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "FromPlayer".encode(s)));
                try!(s.emit_struct_field("pos", 1, |s| pos.encode(s)));
                try!(s.emit_struct_field("event", 2, |s| event.encode(s)));
                Ok(())
            }),
            &EventType::NewGame { first, ref hands } => s.emit_struct("Event", 3, |s| {
                // Should rarely happen
                try!(s.emit_struct_field("type", 0, |s| "NewGameGlobal".encode(s)));
                try!(s.emit_struct_field("first", 1, |s| first.encode(s)));
                try!(s.emit_struct_field("hands", 2, |s| hands.encode(s)));
                Ok(())
            }),
            &EventType::NewGameRelative { first, ref hand } => s.emit_struct("Event", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "NewGame".encode(s)));
                try!(s.emit_struct_field("first", 1, |s| first.encode(s)));
                try!(s.emit_struct_field("cards", 2, |s| hand.encode(s)));
                Ok(())
            }),
            &EventType::GameOver { points, winner, scores } => s.emit_struct("Event", 4, |s| {
                try!(s.emit_struct_field("type", 0, |s| "GameOver".encode(s)));
                try!(s.emit_struct_field("points", 1, |s| points.encode(s)));
                try!(s.emit_struct_field("winner", 2, |s| winner.encode(s)));
                try!(s.emit_struct_field("scores", 3, |s| scores.encode(s)));
                Ok(())
            }),
        }
    }
}

/// Represents an event happening to the game.
#[derive(Clone,RustcEncodable)]
pub struct Event {
    /// Actual event
    pub event: EventType,
    /// Event ID
    pub id: usize,
}
