use super::pos;
use super::cards;
use super::points;

pub struct Trick {
    pub cards: [cards::Card; 4],
    pub first: pos::PlayerPos,
}

impl Trick {
    pub fn score(&self, trump: cards::Suit) -> i32 {
        let mut score = 0;
        for card in self.cards.iter() { score += points::score(*card, trump); }
        score
    }

    pub fn winner(&self, trump: cards::Suit, current: pos::PlayerPos) -> pos::PlayerPos {
        // For every player between 
        let mut best = self.first;
        let mut best_strength = 0;
        // Iterate on every player between the first and the current, excluded
        for pos in self.first.until(current) {
            let strength = points::strength(self.cards[pos.0], trump);
            if strength > best_strength {
                best_strength = strength;
                best = pos;
            }
        }

        best
    }
}

