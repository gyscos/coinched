//!

use super::libcoinche;

pub fn handle_game() {
    let mut a = libcoinche::bid::new_auction(libcoinche::pos::P0);

    while a.get_state() != libcoinche::bid::AuctionState::Over {

        // Get the actual action from player connector
        a.pass();

    }
}
