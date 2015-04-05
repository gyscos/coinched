//! Module for the card game, after auctions are complete.

use super::pos;
use super::cards;
use super::trick;
use super::bid;
use super::points;

// GameState describes the state of a coinche game, ready to play a card.
pub struct GameState {
    players: [cards::Hand; 4],

    current: pos::PlayerPos,

    contract: bid::Contract,

    scores: [i32; 2],
    tricks: Vec<trick::Trick>,
}


pub fn new_game(first: pos::PlayerPos, hands: [cards::Hand; 4], contract: bid::Contract) -> GameState {
    // Create a new game, deal cards to each player
    GameState {
        players: hands,
        current: first,
        contract: contract,
        tricks: vec![trick::empty_trick(first)],
        scores: [0; 2],
    }
}

#[derive(PartialEq,Debug)]
pub enum TrickResult {
    Nothing,
    TrickOver(pos::PlayerPos),
}

#[derive(PartialEq,Debug)]
pub enum PlayError {
    TurnError,
    CardMissing,
    IncorrectSuit,
    InvalidPiss,
    NonRaisedTrump,
}

impl GameState {

    pub fn scores(&self) -> [i32; 2] {
        self.scores
    }

    pub fn play_card(&mut self, player: pos::PlayerPos, card: cards::Card)
                     -> Result<TrickResult,PlayError> {
        if self.current != player {
            return Err(PlayError::TurnError);
        }

        // Is that a valid move?
        try!(self.can_play(player, card));

        // Play the card
        let trump = self.contract.trump;
        let over = self.current_trick_mut().play_card(player, card, trump);

        // Is the trick over?
        let result: TrickResult;
        if over {
            let winner = self.current_trick().winner;
            let score = self.current_trick().score(trump);
            self.scores[winner.team().0] += score;
            self.tricks.push(trick::empty_trick(winner));
            result = TrickResult::TrickOver(winner);
        } else {
            result = TrickResult::Nothing;
        }

        // Next!
        self.current = self.current.next();

        Ok(result)
    }

    pub fn can_play(&self, p: pos::PlayerPos, card: cards::Card) -> Result<(),PlayError> {
        let hand = self.players[p.0];

        // First, we need the card to be able to play
        if !hand.has(card) {
            return Err(PlayError::CardMissing);;
        }

        if p == self.current_trick().first {
            return Ok(());
        }

        let card_suit = card.suit();
        let starting_suit = self.current_trick().cards[self.current_trick().first.0].suit();
        if card_suit != starting_suit {
            if hand.has_any(starting_suit) {
                return Err(PlayError::IncorrectSuit);
            }

            if card_suit != self.contract.trump {
                let partner_winning = p.is_partner(self.current_trick().winner);
                if !partner_winning && hand.has_any(self.contract.trump) {
                    return Err(PlayError::InvalidPiss);
                }
            }
        }

        // One must raise when playing trump
        if card_suit == self.contract.trump {
            let highest = highest_trump(&self.current_trick(), self.contract.trump, p);
            if points::trump_strength(card.rank()) < highest {
                if has_higher(hand, card_suit, highest) {
                    return Err(PlayError::NonRaisedTrump);;
                }
            }
        }

        Ok(())
    }

    fn current_trick(&self) -> &trick::Trick {
        let i = self.tricks.len()-1;
        &self.tricks[i]
    }

    fn current_trick_mut(&mut self) -> &mut trick::Trick {
        let i = self.tricks.len()-1;
        &mut self.tricks[i]
    }
}

#[test]
fn test_play_card() {
    let mut hands = [cards::new_hand(); 4];
    hands[0].add(cards::make_card(cards::HEART, cards::RANK_8));
    hands[0].add(cards::make_card(cards::HEART, cards::RANK_X));
    hands[0].add(cards::make_card(cards::HEART, cards::RANK_A));
    hands[0].add(cards::make_card(cards::HEART, cards::RANK_9));
    hands[0].add(cards::make_card(cards::CLUB, cards::RANK_7));
    hands[0].add(cards::make_card(cards::CLUB, cards::RANK_8));
    hands[0].add(cards::make_card(cards::CLUB, cards::RANK_9));
    hands[0].add(cards::make_card(cards::CLUB, cards::RANK_J));

    hands[1].add(cards::make_card(cards::CLUB, cards::RANK_Q));
    hands[1].add(cards::make_card(cards::CLUB, cards::RANK_K));
    hands[1].add(cards::make_card(cards::CLUB, cards::RANK_X));
    hands[1].add(cards::make_card(cards::CLUB, cards::RANK_A));
    hands[1].add(cards::make_card(cards::SPADE, cards::RANK_7));
    hands[1].add(cards::make_card(cards::SPADE, cards::RANK_8));
    hands[1].add(cards::make_card(cards::SPADE, cards::RANK_9));
    hands[1].add(cards::make_card(cards::SPADE, cards::RANK_J));

    hands[2].add(cards::make_card(cards::DIAMOND, cards::RANK_7));
    hands[2].add(cards::make_card(cards::DIAMOND, cards::RANK_8));
    hands[2].add(cards::make_card(cards::DIAMOND, cards::RANK_9));
    hands[2].add(cards::make_card(cards::DIAMOND, cards::RANK_J));
    hands[2].add(cards::make_card(cards::SPADE, cards::RANK_Q));
    hands[2].add(cards::make_card(cards::SPADE, cards::RANK_K));
    hands[2].add(cards::make_card(cards::HEART, cards::RANK_Q));
    hands[2].add(cards::make_card(cards::HEART, cards::RANK_K));

    hands[3].add(cards::make_card(cards::DIAMOND, cards::RANK_Q));
    hands[3].add(cards::make_card(cards::DIAMOND, cards::RANK_K));
    hands[3].add(cards::make_card(cards::DIAMOND, cards::RANK_X));
    hands[3].add(cards::make_card(cards::DIAMOND, cards::RANK_A));
    hands[3].add(cards::make_card(cards::SPADE, cards::RANK_X));
    hands[3].add(cards::make_card(cards::SPADE, cards::RANK_A));
    hands[3].add(cards::make_card(cards::HEART, cards::RANK_7));
    hands[3].add(cards::make_card(cards::HEART, cards::RANK_J));

    let contract = bid::Contract {
        trump: cards::HEART,
        author: pos::P0,
        target: bid::Target::Contract80,
        coinche_level: 0,
    };

    let mut game = new_game(pos::P0, hands, contract);

    // Wrong turn
    assert_eq!(
        game.play_card(pos::P1, cards::make_card(cards::CLUB, cards::RANK_X)).err(),
        Some(PlayError::TurnError));
    assert_eq!(
        game.play_card(pos::P0, cards::make_card(cards::CLUB, cards::RANK_7)).ok(),
        Some(TrickResult::Nothing));
    // Card missing
    assert_eq!(
        game.play_card(pos::P1, cards::make_card(cards::HEART, cards::RANK_7)).err(),
        Some(PlayError::CardMissing));
    // Wrong color
    assert_eq!(
        game.play_card(pos::P1, cards::make_card(cards::SPADE, cards::RANK_7)).err(),
        Some(PlayError::IncorrectSuit));
    assert_eq!(
        game.play_card(pos::P1, cards::make_card(cards::CLUB, cards::RANK_Q)).ok(),
        Some(TrickResult::Nothing));
    // Invalid piss
    assert_eq!(
        game.play_card(pos::P2, cards::make_card(cards::DIAMOND, cards::RANK_7)).err(),
        Some(PlayError::InvalidPiss));
    assert_eq!(
        game.play_card(pos::P2, cards::make_card(cards::HEART, cards::RANK_Q)).ok(),
        Some(TrickResult::Nothing));
    // UnderTrump
    assert_eq!(
        game.play_card(pos::P3, cards::make_card(cards::HEART, cards::RANK_7)).err(),
        Some(PlayError::NonRaisedTrump));
    assert_eq!(
        game.play_card(pos::P3, cards::make_card(cards::HEART, cards::RANK_J)).ok(),
        Some(TrickResult::TrickOver(pos::P3)));
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
