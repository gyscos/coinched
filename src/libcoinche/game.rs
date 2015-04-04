//! Module for the card game, after auctions are complete.

use super::pos;
use super::cards;
use super::trick;
use super::bid;
use super::points;

// GameState describes the state of a coinche game, ready to play a card.
pub struct GameState {
    pub players: [cards::Hand; 4],
    pub current: pos::PlayerPos,

    pub contract: bid::Contract,

    pub current_trick: trick::Trick,
}


pub fn new_game(first: pos::PlayerPos, hands: [cards::Hand; 4], contract: bid::Contract) -> GameState {
    // Create a new game, deal cards to each player
    GameState {
        players: hands,
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

#[test]
fn test_has_higher_1() {
    // Simple case: X is always higher than Q.
    let mut hand = cards::new_hand();

    hand.add(cards::make_card(cards::HEART, cards::RANK_8));
    hand.add(cards::make_card(cards::SPADE, cards::RANK_X));
    assert!(has_higher(hand, cards::SPADE, points::trump_strength(cards::RANK_Q)));
}

#[test]
fn test_has_higher_2() {
    // Test that we don't mix colors
    let mut hand = cards::new_hand();

    hand.add(cards::make_card(cards::HEART, cards::RANK_8));
    hand.add(cards::make_card(cards::SPADE, cards::RANK_X));
    assert!(!has_higher(hand, cards::HEART, points::trump_strength(cards::RANK_Q)));
}

#[test]
fn test_has_higher_3() {
    // In the trump order, X is lower than 9
    let mut hand = cards::new_hand();

    hand.add(cards::make_card(cards::HEART, cards::RANK_J));
    hand.add(cards::make_card(cards::SPADE, cards::RANK_X));
    assert!(!has_higher(hand, cards::SPADE, points::trump_strength(cards::RANK_9)));
}

#[test]
fn test_has_higher_4() {
    // In the trump order, J is higher than A
    let mut hand = cards::new_hand();

    hand.add(cards::make_card(cards::HEART, cards::RANK_8));
    hand.add(cards::make_card(cards::SPADE, cards::RANK_J));
    assert!(has_higher(hand, cards::SPADE, points::trump_strength(cards::RANK_A)));
}

#[test]
fn test_has_higher_5() {
    // Test when we have no trump at all
    let mut hand = cards::new_hand();

    hand.add(cards::make_card(cards::HEART, cards::RANK_J));
    hand.add(cards::make_card(cards::DIAMOND, cards::RANK_J));
    hand.add(cards::make_card(cards::SPADE, cards::RANK_J));
    assert!(!has_higher(hand, cards::CLUB, points::trump_strength(cards::RANK_7)));
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
