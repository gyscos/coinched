use super::{AuctionAction, Backend, Frontend, GameAction};
use libcoinche::{cards, pos};
use {CardBody, ContractBody, EventType, PlayerEvent};

pub struct Client<B, F> {
    pub scores: [i32; 2],
    backend: B,
    frontend: F,
}

enum GameError {
    NoContract,
    PlayerLeft,
}

impl<B, F> Client<B, F>
where
    B: Backend,
    F: Frontend<B>,
{
    pub fn new(backend: B, frontend: F) -> Self {
        Client {
            scores: [0, 0],
            backend,
            frontend,
        }
    }

    pub fn run<F: Frontend<B>>(mut self) -> [i32; 2] {
        loop {
            match self.backend.wait() {
                Ok(EventType::NewGameRelative { first, hand }) => {
                    match self.run_game(first, hand) {
                        Err(GameError::PlayerLeft) => return self.scores,
                        _ => (),
                    }
                }
                Ok(event) => self.frontend.unexpected_event(event),
                Err(err) => self.frontend.show_error(err),
            }
        }
    }

    fn run_game(&mut self, first: pos::PlayerPos, hand: cards::Hand) -> Result<(), GameError> {
        self.frontend.start_game(first, hand);
        try!(self.run_auction());
        try!(self.run_cardgame());
        Ok(())
    }

    fn run_auction(&mut self) -> Result<(), GameError> {
        loop {
            let mut event = self.backend.wait();
            match event {
                Ok(EventType::YourTurn) => {
                    event = match self.frontend.ask_bid() {
                        AuctionAction::Leave => {
                            self.frontend.party_cancelled("you left");
                            return Err(GameError::PlayerLeft);
                        }
                        AuctionAction::Coinche => self.backend.coinche(),
                        AuctionAction::Pass => self.backend.pass(),
                        AuctionAction::Bid((suit, target)) => self.backend.bid(ContractBody {
                            suit: suit,
                            target: target,
                        }),
                    }
                }
                _ => (),
            }

            match event {
                Ok(EventType::FromPlayer(pos, e)) => match e {
                    PlayerEvent::Bidded(suit, target) => self.frontend.show_bid(pos, suit, target),
                    PlayerEvent::Passed => self.frontend.show_pass(pos),
                    PlayerEvent::Coinched => self.frontend.show_coinche(pos),
                    _ => self
                        .frontend
                        .unexpected_event(EventType::FromPlayer(pos, e)),
                },
                Ok(EventType::BidCancelled) => {
                    self.frontend.auction_cancelled();
                    return Err(GameError::NoContract);
                }
                Ok(EventType::PartyCancelled(msg)) => {
                    self.frontend.party_cancelled(&msg);
                    return Err(GameError::PlayerLeft);
                }
                Ok(EventType::BidOver(contract)) => {
                    self.frontend.auction_over(&contract);
                    return Ok(());
                }
                Ok(event) => self.frontend.unexpected_event(event),
                Err(err) => self.frontend.show_error(err),
            }
        }
    }

    fn run_cardgame(&mut self) -> Result<(), GameError> {
        loop {
            let mut event = self.backend.wait();
            match event {
                Ok(EventType::YourTurn) => {
                    event = match self.frontend.ask_card() {
                        GameAction::Leave => {
                            self.frontend.party_cancelled("you left");
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
                Ok(EventType::GameOver {
                    points,
                    winner,
                    scores,
                }) => {
                    self.scores[0] += scores[0];
                    self.scores[1] += scores[1];
                    self.frontend.game_over(points, winner, scores);
                    return Ok(());
                }
                Ok(EventType::TrickOver { winner }) => self.frontend.show_trick_over(winner),
                Ok(EventType::FromPlayer(pos, e)) => match e {
                    PlayerEvent::CardPlayed(card) => self.frontend.show_card_played(pos, card),
                    _ => self
                        .frontend
                        .unexpected_event(EventType::FromPlayer(pos, e)),
                },
                Ok(EventType::PartyCancelled(msg)) => {
                    self.frontend.party_cancelled(&msg);
                    return Err(GameError::PlayerLeft);
                }
                Ok(event) => self.frontend.unexpected_event(event),
                Err(err) => self.frontend.show_error(err),
            }
        }
    }
}
