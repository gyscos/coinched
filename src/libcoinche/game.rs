//! Module for the card game, after auctions are complete.

use super::pos;
use super::cards;
use super::trick;
use super::bid;
use super::points;

// GameState describes the state of a coinche game.
pub struct GameState {
    pub players: [cards::Hand; 4],
    pub current: pos::PlayerPos,

    pub contract: bid::Contract,

    pub current_trick: trick::Trick,
}


pub fn new_game(first: pos::PlayerPos, contract: bid::Contract) -> GameState {
    // Create a new game, deal cards to each player
    GameState {
        players: super::deal_hands(),
        current: first,
        contract: contract,
        current_trick: trick::empty_trick(first),
    }
}

impl GameState {
    pub fn play_card(&mut self, player: pos::PlayerPos, card: cards::Card) {
        if self.current != player {
            return
        }

        if !self.can_play(player, card) {
            return
        }
    }

    pub fn can_play(&self, p: pos::PlayerPos, card: cards::Card) -> bool {
        let hand = self.players[p.0];
        if !hand.has(card) {
            return false;
        }

        if p == self.current_trick.first {
            return true
        }

        let card_suit = card.suit();
        if card_suit == self.contract.trump {
            let highest = highest_trump(&self.current_trick, self.contract.trump, p);
            if points::trump_strength(card.rank()) < highest {
                if has_higher(hand, card_suit, highest) {
                    return false;
                }
            }
        }

        true
    }
}

fn has_higher(hand: cards::Hand, trump: cards::Suit, strength: i32) -> bool {
    for ri in 0..8 {
        let rank = cards::get_rank(ri);
        if points::trump_strength(rank) > strength && hand.has(cards::make_card(trump, rank)) {
            return true
        }
    }

    false
}

fn highest_trump(trick: &trick::Trick, trump: cards::Suit, player: pos::PlayerPos) -> i32 {
    let mut highest = -1;

    for p in trick.first.until(player) {
        if trick.cards[p.0].suit() == trump {
            let str = points::trump_strength(trick.cards[p.0].rank());
            if str > highest {
                highest = str;
            }
        }
    }

    highest
}
