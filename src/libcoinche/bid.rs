use super::game;
use super::cards;
use super::pos;

pub enum Target {
    Contract80,
    Contract90,
    Contract100,
    Contract110,
    Contract120,
    Contract130,
    Contract140,
    Contract150,
    Contract160,
    ContractCapot,
}

impl Target {
    pub fn score(&self) -> i32 {
        match *self {
            Target::Contract80 => 80,
            Target::Contract90 => 90,
            Target::Contract100 => 100,
            Target::Contract110 => 110,
            Target::Contract120 => 120,
            Target::Contract130 => 130,
            Target::Contract140 => 140,
            Target::Contract150 => 150,
            Target::Contract160 => 160,
            Target::ContractCapot => 250,
        }
    }

    pub fn victory(&self) -> bool {
        false
    }
}

pub struct Contract {
    pub trump: cards::Suit,
    pub author: pos::PlayerPos,
    pub target: Target,
}

pub struct Auction {
    history: Vec<Contract>,
    pass_count: i32,
}

impl Auction {
    pub fn bid(&mut self, contract: Contract) -> bool {
        self.history.push(contract);

        false
    }

    pub fn pass(&mut self) -> bool {
        self.pass_count += 1;

        // After 3 passes, we're back to the contract author, and we can start.
        (self.pass_count == 3)
    }

    // Moves the auction to kill it
    pub fn complete(mut self, first: pos::PlayerPos) -> game::GameState {
        game::new_game(first, self.history.pop().expect("contract history empty"))
    }
}

