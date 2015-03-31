use super::PlayerPos;
use super::cards;

// GameState describes the state of a coinche game.
pub struct GameState {
    pub players: [cards::Hand; 4],
    pub current: PlayerPos,

    pub trump: cards::Suit,
}

pub fn new_game() -> GameState {
    // Create a new game, deal cards to each player
    GameState {
        players: cards::deal_hands(),
        current: PlayerPos(0),
        trump: cards::Suit(0),
    }
}


impl GameState {
    pub fn play_card(&mut self, player: PlayerPos, card: cards::Card) {
        if self.current != player {
            return
        }
    }
}

