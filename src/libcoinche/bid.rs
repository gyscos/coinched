use super::PlayerPos;
use super::game;

pub struct Auction {
    pub current: PlayerPos,
}

impl Auction {
    pub fn complete(&self) -> game::GameState {
        game::new_game()
    }
}

