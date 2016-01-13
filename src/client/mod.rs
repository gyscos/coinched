use libcoinche::{pos, bid, cards};
use {EventType, ContractBody, CardBody};

pub mod http;
mod client;

pub use self::client::Client;

pub enum AuctionAction {
    Leave,
    Pass,
    Coinche,
    Bid((cards::Suit, bid::Target)),
}

pub trait Frontend<B: Backend> {
    fn show_error(&mut self, error: B::Error);
    fn unexpected_event(&mut self, event: EventType);
    fn party_cancelled(self, msg: String);
}

pub trait GameFrontend<B: Backend> : Frontend<B> {
}

pub trait AuctionFrontend<B: Backend> : Frontend<B> {
    type Game: GameFrontend<B>;

    fn show_pass(&mut self, pos: pos::PlayerPos);
    fn show_coinche(&mut self, pos: pos::PlayerPos);
    fn show_bid(&mut self, pos: pos::PlayerPos, suit: cards::Suit, target: bid::Target);

    fn ask_action(&mut self) -> AuctionAction;

    /// Auction cancelled, back to the start.
    fn auction_cancelled(self);
    /// Auction is complete, we can play now!
    fn auction_over(self, contract: &bid::Contract) -> Self::Game;

}

pub trait MainFrontend<B: Backend> : Frontend<B> {
    type Auction: AuctionFrontend<B>;

    fn start_game(&mut self, first: pos::PlayerPos, hand: cards::Hand) -> Self::Auction;
}

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
