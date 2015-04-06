use super::Connector;
use super::game_handler::handle_game;

pub fn handle_party(mut connector: Connector) {
    handle_game(&mut connector);
}
