//!

use super::libcoinche;
use super::{ActionResult,Connector};

pub fn handle_game(connector: &mut Connector) {
    match handle_bidding(connector) {
        // Play the game!
        Ok(game) => handle_cardplay(connector, game),
        // Cancelled... try again.
        Err(_) => (),
    }
}

fn handle_bidding(connector: &mut Connector) -> Result<libcoinche::GameState,libcoinche::bid::BidError> {
    let mut current = libcoinche::pos::P0;

    let mut a = libcoinche::new_auction(current);

    while a.get_state() != libcoinche::AuctionState::Over {

        match connector.input.recv() {
            Ok(_) => (),
            Err(_) => (),
        }

        // 
        // Get the actual action from player connector
        a.pass();
        current = current.next();
    }

    a.complete()
}

fn handle_cardplay(connector: &mut Connector, game: libcoinche::GameState) {
    let mut current = libcoinche::pos::P0;

    // A game in 8 tricks
    for _ in 0..8 {
        loop {
            match connector.input.recv() {
                Ok(_) => (),
                Err(_) => (),
            }
            current = current.next();
        }
    }

    // Send them scores now I guess?
    let scores = game.scores();
    for sender in connector.output.iter() {
        // The receiver should always be present, right?
        sender.send(ActionResult::GameOver(scores)).unwrap();
    }
}
