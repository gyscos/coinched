use libcoinche::{cards, pos};
use {PlayerEvent, EventType, ContractBody};
use super::{Backend, MainFrontend, AuctionAction, AuctionFrontend, Frontend};

pub struct Client<B: Backend, F: MainFrontend<B>> {
    pub scores: [i32; 2],
    backend: B,
    frontend: F,
}

enum GameError {
    NoContract,
    PlayerLeft,
}


impl<B: Backend, F: MainFrontend<B>> Client<B, F> {
    pub fn new(backend: B, frontend: F) -> Self {
        Client {
            scores: [0, 0],
            backend: backend,
            frontend: frontend,
        }
    }

    pub fn run(mut self) -> [i32; 2] {
        loop {
            match self.backend.wait() {
                Ok(EventType::NewGameRelative {first, hand}) => {
                    let scores = match self.run_game(first, hand) {
                        Ok(scores) => scores,
                        Err(GameError::NoContract) => continue,
                        Err(GameError::PlayerLeft) => return self.scores,
                    };
                    self.scores[0] += scores[0];
                    self.scores[1] += scores[1];
                }
                Ok(_) => (),
                Err(err) => {
                    self.frontend.show_error(err);
                    continue;
                }
            }
        }
    }

    fn run_game(&mut self,
                first: pos::PlayerPos,
                hand: cards::Hand)
                -> Result<[i32; 2], GameError> {
        let auction = self.frontend.start_game(first, hand);
        let game = try!(self.run_auction(auction));
        Ok(self.run_cardgame(game))
    }

    // God that's an ugly type. Really, I want `F::Auction::Game`.
    fn run_auction(&mut self,
                   mut auction: F::Auction)
                   -> Result<<F::Auction as AuctionFrontend<B>>::Game, GameError> {
        loop {
            let mut event = self.backend.wait();
            match event {
                Ok(EventType::YourTurn) => {
                    event = match auction.ask_action() {
                        AuctionAction::Leave => {
                            auction.auction_cancelled();
                            return Err(GameError::PlayerLeft);
                        }
                        AuctionAction::Coinche => self.backend.coinche(),
                        AuctionAction::Pass => self.backend.pass(),
                        AuctionAction::Bid((suit, target)) => {
                            self.backend.bid(ContractBody {
                                suit: suit,
                                target: target,
                            })
                        }
                    }
                }
                _ => (),
            }

            match event {
                Ok(EventType::FromPlayer(pos, e)) => {
                    match e {
                        PlayerEvent::Bidded(suit, target) => auction.show_bid(pos, suit, target),
                        PlayerEvent::Passed => auction.show_pass(pos),
                        PlayerEvent::Coinched => auction.show_coinche(pos),
                        _ => auction.unexpected_event(EventType::FromPlayer(pos, e)),
                    }
                }
                Ok(EventType::BidCancelled) => {
                    auction.auction_cancelled();
                    return Err(GameError::NoContract);
                }
                Ok(EventType::PartyCancelled(msg)) => {
                    auction.party_cancelled(msg);
                    return Err(GameError::PlayerLeft);
                }
                Ok(EventType::BidOver(contract)) => return Ok(auction.auction_over(&contract)),
                Ok(event) => auction.unexpected_event(event),
                Err(err) => auction.show_error(err),
            }
        }
    }

    fn run_cardgame(&mut self, game: <F::Auction as AuctionFrontend<B>>::Game) -> [i32; 2] {
        [0, 0]
    }
}
