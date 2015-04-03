use super::game;
use super::cards;
use super::pos;

#[derive(PartialEq)]
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

    pub fn victory(&self, score: i32, capot: bool) -> bool {
        match *self {
            Target::Contract80 => score >= 80,
            Target::Contract90 => score >= 90,
            Target::Contract100 => score >= 100,
            Target::Contract110 => score >= 110,
            Target::Contract120 => score >= 120,
            Target::Contract130 => score >= 130,
            Target::Contract140 => score >= 140,
            Target::Contract150 => score >= 150,
            Target::Contract160 => score >= 160,
            Target::ContractCapot => capot,
        }
    }
}

pub struct Contract {
    pub trump: cards::Suit,
    pub author: pos::PlayerPos,
    pub target: Target,
    pub coinche_level: i32,
}

pub struct Auction {
    history: Vec<Contract>,
    pass_count: i32,
    stopped: bool,
}

pub fn new_auction() -> Auction {
    Auction {
        history: Vec::new(),
        pass_count: 0,
        stopped: false,
    }
}

impl Auction {
    // Bid a new, higher contract.
    pub fn bid(&mut self, contract: Contract) -> Result<bool,String> {
        if self.stopped {
            return Err("auction is closed".to_string());
        }

        if !self.history.is_empty() {
            if contract.target.score() < self.history[self.history.len()-1].target.score() {
                return Err("must bid higher than current contract".to_string());
            }
        }

        self.stopped = contract.target == Target::ContractCapot;

        self.history.push(contract);
        self.pass_count = 0;

        // Only stops the bids if the guy asked for a capot
        Ok(self.stopped)
    }

    pub fn pass(&mut self) -> bool {
        self.pass_count += 1;

        // After 3 passes, we're back to the contract author, and we can start.
        self.stopped = self.pass_count == 3;

        self.stopped
    }

    pub fn coinche(&mut self) -> Result<bool,String> {
        if self.history.is_empty() {
            Err("no contract to coinche".to_string())
        } else {
            self.stopped = true;
            let i = self.history.len() - 1;
            if self.history[i].coinche_level > 1 {
                Err("constract is already sur-coinched".to_string())
            } else {
                self.history[i].coinche_level += 1;
                // Stop if we are already sur-coinching
                Ok(self.history[i].coinche_level == 2)
            }
        }
    }

    // Moves the auction to kill it
    pub fn complete(mut self, first: pos::PlayerPos) -> Result<game::GameState,String> {
        if !self.stopped {
            Err("auction is still running".to_string())
        } else if self.history.is_empty() {
            Err("no contract to start the game with".to_string())
        } else {
            Ok(game::new_game(first, self.history.pop().expect("contract history empty")))
        }
    }
}

#[test]
fn test_auction() {
    let mut auction = new_auction();

    assert!(!auction.stopped);

    assert!(!auction.pass());
    assert!(!auction.pass());
}
