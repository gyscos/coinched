use std::collections::HashMap;
use std::sync::{Arc,RwLock,mpsc};

use super::libcoinche::{bid,cards,pos,game};

pub enum ServerError {
    BadPlayerId,
    PlayInAuction,
    BidInGame,
    Bid(bid::BidError),
    Play(game::PlayError),
}

pub enum Action {
    Bid(bid::Contract),
    Coinche,
    Pass,
    Play(cards::Card),
}

#[derive(Clone)]
pub enum PlayerEvent {
    Bidded(bid::Contract),
    Coinched,
    Passed,
    CardPlayed(cards::Card),
}

#[derive(Clone)]
pub enum Event {
    PlayerEvent(pos::PlayerPos, PlayerEvent),

    // Bid over: contains the contract and the author
    BidOver(pos::PlayerPos, bid::Contract),
    BidCancelled,

    // Trick over: contains the winner
    TrickOver(pos::PlayerPos),

    // New game: contains the first player, and the player's hand
    NewGame(pos::PlayerPos, cards::Hand),

    // Game over: contains scores
    GameOver([i32;2], pos::Team, [i32;2]),

    // The party is cancelled. Contains an optional explanation.
    PartyCancelled(String),
}

pub struct Order {
    pub author: pos::PlayerPos,
    pub action: Action
}

pub struct Server {
    party_list: RwLock<PartyList>,
}

pub enum Game {
    Bidding(bid::Auction),
    Playing(game::GameState),
}

pub struct Party {
    game: Game,
    events: Vec<Event>,
    observers: Vec<mpsc::Sender<Event>>,
}

pub struct PlayerInfo {
    pub party: Arc<RwLock<Party>>,
    pub pos: pos::PlayerPos,
}

pub struct PartyList {
    pub player_map: HashMap<u32,PlayerInfo>,
}

pub fn play_card(server: &Server, player_id: u32, card: cards::Card) -> Result<Event,ServerError> {
    let list = server.party_list.read().unwrap();

    if !list.player_map.contains_key(&player_id) {
        // Bad player id!

        return Err(ServerError::BadPlayerId);
    }

    let info = list.player_map.get(&player_id).unwrap();

    let mut party = info.party.write().unwrap();

    match &mut party.game {
        &mut Game::Bidding(_) => {
            // Bad time for a card play!
            return Err(ServerError::PlayInAuction);
        },
        &mut Game::Playing(ref mut game) => {
            match game.play_card(info.pos, card) {
                Ok(result) => {
                    // Ok, propagate the event
                },
                Err(err) => {
                    // Nothing to see here
                    return Err(ServerError::Play(err));
                },
            }
        },
    }

    // Dummy event before handling the real case
    Ok(Event::BidCancelled)
}

enum WaitResult {
    Ready(Event),
    Waiting(mpsc::Receiver<Event>),
}

pub fn wait(server: &Server, player_id: u32, event_id: usize) -> Result<Event,ServerError> {
    let res = get_wait_result(server, player_id, event_id);

    let event = match res {
        None => return Err(ServerError::BadPlayerId),
        Some(WaitResult::Ready(event)) => event,
        Some(WaitResult::Waiting(rx)) => rx.recv().unwrap(),
    };

    Ok(event)
}

fn get_wait_result(server: &Server, player_id: u32, event_id: usize) -> Option<WaitResult> {
    let list = server.party_list.read().unwrap();
    if !list.player_map.contains_key(&player_id) {
        return None;
    }

    let info = list.player_map.get(&player_id).unwrap();
    let mut party = info.party.write().unwrap();

    if party.events.len() >= event_id {
        return Some(WaitResult::Ready(party.events[event_id].clone()));
    }

    let (tx, rx) = mpsc::channel();
    party.observers.push(tx);

    Some(WaitResult::Waiting(rx))
}
