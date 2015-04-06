pub mod libcoinche;
pub mod game_handler;
pub mod party_handler;

use std::sync::mpsc;

pub enum Action {
    Pass,
    Play(libcoinche::cards::Card),
    Bid(libcoinche::bid::Contract),
}

pub enum ActionResult {
    Success,

    // Trick over: contains the winner
    TrickOver(libcoinche::pos::PlayerPos),

    // Game over: contains scores
    GameOver([i32;2]),
}

pub struct Order {
    pub author: libcoinche::pos::PlayerPos,
    pub action: Action
}

pub struct Connector {
    input: mpsc::Receiver<Order>,
    output: [mpsc::Sender<ActionResult>; 4],
}
