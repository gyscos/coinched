use libcoinche::{cards, pos};
use {PlayerEvent, EventType, ContractBody, CardBody};
use super::{Backend, AuctionAction, Frontend, GameAction};

pub struct Client<B: Backend> {
    pub scores: [i32; 2],
    backend: B,
}

enum GameError {
    NoContract,
    PlayerLeft,
}


impl<B: Backend> Client<B> {
    pub fn new(backend: B) -> Self {
        Client {
            scores: [0, 0],
            backend: backend,
        }
    }

    pub fn run<F: Frontend<B>>(mut self, frontend: &mut F) -> [i32; 2] {
        loop {
            match self.backend.wait() {
                Ok(EventType::NewGameRelative {first, hand}) => {
                    match self.run_game(frontend, first, hand) {
                        Err(GameError::PlayerLeft) => return self.scores,
                        _ => (),
                    }
                }
                Ok(event) => frontend.unexpected_event(event),
                Err(err) => frontend.show_error(err),
            }
        }
    }

    fn run_game<F: Frontend<B>>(&mut self, frontend: &mut F,
                                    first: pos::PlayerPos,
                                    hand: cards::Hand) -> Result<(), GameError> {
        frontend.start_game(first, hand);
        try!(self.run_auction(frontend));
        try!(self.run_cardgame(frontend));
        Ok(())
    }

    // God that's an ugly type. Really, I want `F::Auction::Game`.
    fn run_auction<F: Frontend<B>>(&mut self, frontend: &mut F) -> Result<(), GameError> {
        loop {
            let mut event = self.backend.wait();
            match event {
                Ok(EventType::YourTurn) => {
                    event = match frontend.ask_bid() {
                        AuctionAction::Leave => {
                            frontend.party_cancelled("you left");
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
                        PlayerEvent::Bidded(suit, target) => frontend.show_bid(pos, suit, target),
                        PlayerEvent::Passed => frontend.show_pass(pos),
                        PlayerEvent::Coinched => frontend.show_coinche(pos),
                        _ => frontend.unexpected_event(EventType::FromPlayer(pos, e)),
                    }
                }
                Ok(EventType::BidCancelled) => {
                    frontend.auction_cancelled();
                    return Err(GameError::NoContract);
                }
                Ok(EventType::PartyCancelled(msg)) => {
                    frontend.party_cancelled(&msg);
                    return Err(GameError::PlayerLeft);
                }
                Ok(EventType::BidOver(contract)) => {
                    frontend.auction_over(&contract);
                    return Ok(());
                }
                Ok(event) => frontend.unexpected_event(event),
                Err(err) => frontend.show_error(err),
            }
        }
    }

    fn run_cardgame<F: Frontend<B>>(&mut self, frontend: &mut F) -> Result<(), GameError> {
        loop {
            let mut event = self.backend.wait();
            match event {
                Ok(EventType::YourTurn) => {
                    event = match frontend.ask_card() {
                        GameAction::Leave => {
                            frontend.party_cancelled("you left");
                            return Err(GameError::PlayerLeft);
                        }
                        GameAction::PlayCard(card) => {
                            self.backend.play_card(CardBody { card: card })
                        }
                    };
                }
                _ => (),
            }

            match event {
                Ok(EventType::GameOver{points, winner, scores}) => {
                    self.scores[0] += scores[0];
                    self.scores[1] += scores[1];
                    frontend.game_over(points, winner, scores);
                    return Ok(());
                }
                Ok(EventType::TrickOver{winner}) => frontend.show_trick_over(winner),
                Ok(EventType::FromPlayer(pos, e)) => {
                    match e {
                        PlayerEvent::CardPlayed(card) => frontend.show_card_played(pos, card),
                        _ => frontend.unexpected_event(EventType::FromPlayer(pos, e)),
                    }
                }
                Ok(EventType::PartyCancelled(msg)) => {
                    frontend.party_cancelled(&msg);
                    return Err(GameError::PlayerLeft);
                }
                Ok(event) => frontend.unexpected_event(event),
                Err(err) => frontend.show_error(err),
            }
        }
    }
}
