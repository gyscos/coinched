//! Event module

use libcoinche::{bid, cards, pos};

/// An event about a player.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerEvent {
    /// A player made a new bid in the auction.
    Bidded {
        suit: cards::Suit,
        target: bid::Target,
    },

    /// A player coinched the current bid in the auction.
    Coinched,

    /// A player passed in the auction.
    Passed,

    /// A player played a card.
    CardPlayed { card: cards::Card },
}

/// Represents an event that can happen during the game.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    /// Special event indicating the server expects the player to take an action.
    YourTurn,

    /// The party is cancelled. Contains an optional explanation.
    PartyCancelled { msg: String },

    /// A player did something!
    FromPlayer {
        pos: pos::PlayerPos,
        event: PlayerEvent,
    },

    /// Bid over: contains the contract and the author
    BidOver { contract: bid::Contract },

    /// The bid was cancelled, probably because no one bidded anything.
    /// A new game is probably on its way.
    BidCancelled,

    /// Trick over: contains the winner
    TrickOver { winner: pos::PlayerPos },

    /// New game: contains the first player, and the player's hand.
    /// For internal use only, it is never sent on the network.
    NewGame {
        first: pos::PlayerPos,
        hands: [cards::Hand; 4],
    },

    /// New game event, translated for each player.
    NewGameRelative {
        first: pos::PlayerPos,
        hand: cards::Hand,
    },

    /// Game over: contains scores
    GameOver {
        points: [i32; 2],
        winner: pos::Team,
        scores: [i32; 2],
    },
}

impl EventType {
    /// Returns a version of the event from the point of view of a given player.
    /// It returns a direct clone of the event for most event types,
    /// except for a NewGame, where it only returns the player's hand.
    pub fn relativize(&self, from: pos::PlayerPos) -> Self {
        match self {
            &EventType::NewGame { first, hands } => EventType::NewGameRelative {
                first: first,
                hand: hands[from as usize],
            },
            _ => self.clone(),
        }
    }
}

/// Represents an event happening to the game.
#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct Event {
    /// Actual event
    pub event: EventType,
    /// Event ID
    pub id: usize,
}
