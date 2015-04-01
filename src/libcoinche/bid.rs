use super::pos;
use super::game;
use super::cards::Suit;

pub struct Contract {
    trump: Suit,
}

pub struct Auction {
    current: pos::PlayerPos,

    contract: Contract,
    history: Vec<Contract>,
}

impl Auction {
    pub fn bid(&mut self, contract: &Contract) {
    }

    // Moves the auction to kill it
    pub fn complete(self) -> game::GameState {
        game::new_game()
    }
}

