use std::fmt;
use std::convert::From;

use libcoinche::bid;
use libcoinche::game;

/// A possible error.
pub enum Error {
    /// The given player ID is not associated with an actual game
    BadPlayerId,
    /// The given event ID is not associated with an actual event
    BadEventId,

    /// Player tried to play a card during auction.
    PlayInAuction,
    /// Player tried to bid during card play.
    BidInGame,

    /// An error occured during bidding.
    Bid(bid::BidError),
    /// An error occured during card play.
    Play(game::PlayError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::BadPlayerId => write!(f, "player not found"),
            &Error::BadEventId => write!(f, "event not found"),
            &Error::PlayInAuction => write!(f, "cannot play during auction"),
            &Error::BidInGame => write!(f, "cannot bid during card play"),
            &Error::Bid(ref error) => write!(f, "{}", error),
            &Error::Play(ref error) => write!(f, "{}", error),
        }
    }
}

impl From<bid::BidError> for Error {
    fn from(err: bid::BidError) -> Error {
        Error::Bid(err)
    }
}
impl From<game::PlayError> for Error {
    fn from(err: game::PlayError) -> Error {
        Error::Play(err)
    }
}
