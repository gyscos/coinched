extern crate rand;

use self::rand::{thread_rng,Rng};

use std::collections::HashMap;
use std::sync::{Arc,RwLock,Mutex,mpsc};

use super::libcoinche::{bid,cards,pos,game};

pub enum ServerError {
    BadPlayerId,
    BadEventId,
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

// Player just joined a new party. He's given a player id, and his position.
pub struct NewPartyInfo {
    pub player_id: u32,
    pub player_pos: pos::PlayerPos,
}

// Represents an event that can happen during the game.
#[derive(Clone)]
pub enum EventType {
    // The party is cancelled. Contains an optional explanation.
    PartyCancelled(String),

    // A player did something!
    FromPlayer(pos::PlayerPos, PlayerEvent),

    // Bid over: contains the contract and the author
    BidOver(pos::PlayerPos, bid::Contract),
    // The bid was cancelled, probably because no one bidded anything.
    // A new game is probably on its way.
    BidCancelled,

    // Trick over: contains the winner
    TrickOver(pos::PlayerPos),

    // New game: contains the first player, and the player's hand
    NewGame(pos::PlayerPos, [cards::Hand;4]),

    // Game over: contains scores
    GameOver([i32;2], pos::Team, [i32;2]),
}

#[derive(Clone)]
pub struct Event {
    pub event: EventType,
    pub id: usize,
}

pub struct Order {
    pub author: pos::PlayerPos,
    pub action: Action
}

pub struct Server {
    party_list: RwLock<PartyList>,

    waiting_list: Mutex<Vec<mpsc::Sender<NewPartyInfo>>>,
}

pub enum Game {
    Bidding(bid::Auction),
    Playing(game::GameState),
}

pub struct Party {
    game: Game,
    events: Vec<EventType>,
    observers: Vec<mpsc::Sender<Event>>,
}

fn new_party(first: pos::PlayerPos) -> Party {
    Party {
        game: Game::Bidding(bid::new_auction(first)),
        events: Vec::new(),
        observers: Vec::new(),
    }
}

impl Party {
    fn add_event(&mut self, event: EventType) {
        let ev = Event{
            event: event.clone(),
            id: self.events.len(),
        };
        for sender in self.observers.iter() {
            sender.send(ev.clone()).unwrap();
        }
        self.observers.clear();
        self.events.push(event);
    }
}

pub struct PlayerInfo {
    pub party: Arc<RwLock<Party>>,
    pub pos: pos::PlayerPos,
}

pub struct PartyList {
    pub player_map: HashMap<u32,PlayerInfo>,
}

impl PartyList {
    fn make_ids(&self) -> [u32; 4] {
        // Expect self.player_map to be locked
        let mut result = [0;4];

        for i in 0..4 {
            loop {
                let id = thread_rng().next_u32();
                if self.player_map.contains_key(&id) {
                    continue;
                }
                let mut ok = true;
                for j in 0..i {
                    if result[j] == id {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    continue;
                }

                result[i] = id;
            }
        }

        result
    }
}

enum WaitResult {
    Ready(Event),
    Waiting(mpsc::Receiver<Event>),
}

enum JoinResult {
    Ready(NewPartyInfo),
    Waiting(mpsc::Receiver<NewPartyInfo>),
}

impl Server {
    pub fn join(&self) -> Option<NewPartyInfo> {
        match self.get_join_result() {
            JoinResult::Ready(info) => Some(info),
            JoinResult::Waiting(rx) => Some(rx.recv().unwrap()),
        }
    }

    fn get_join_result(&self) -> JoinResult {
        let mut waiters = self.waiting_list.lock().unwrap();
        if waiters.len() >= 3 {
            // It's a PARTEY!
            let info = self.make_party([
                                       waiters.pop().unwrap(),
                                       waiters.pop().unwrap(),
                                       waiters.pop().unwrap(),
            ]);
            return JoinResult::Ready(info);
        } else {
            let (tx,rx) = mpsc::channel();
            waiters.push(tx);
            return JoinResult::Waiting(rx);
        }
    }

    fn make_party(&self, others: [mpsc::Sender<NewPartyInfo>; 3]) -> NewPartyInfo {
        let mut list = self.party_list.write().unwrap();

        // Generate 4 new IDS
        let ids = list.make_ids();

        let party = Arc::new(RwLock::new(new_party(pos::P0)));
        // Kickstart it with a new game!

        // Prepare the players info
        for i in 0..4 {
            list.player_map.insert(ids[i], PlayerInfo {
                party: party.clone(),
                pos: pos::PlayerPos(i),
            });
        }

        // Tell everyone. They'll love it.
        for i in 0..3 {
            others[i].send(NewPartyInfo{
                player_id: ids[i],
                player_pos: pos::PlayerPos(i),
            }).unwrap();
        }

        // Even you, weird 4th dude.
        NewPartyInfo{
            player_id: ids[3],
            player_pos: pos::P3,
        }
    }

    // Play a card in the current game
    pub fn play_card(&self, player_id: u32, card: cards::Card) -> Result<Event,ServerError> {
        let list = self.party_list.read().unwrap();

        if !list.player_map.contains_key(&player_id) {
            // Bad player id!

            return Err(ServerError::BadPlayerId);
        }

        let info = list.player_map.get(&player_id).unwrap();

        let mut party = info.party.write().unwrap();

        let result = match &mut party.game {
            &mut Game::Bidding(_) => {
                // Bad time for a card play!
                return Err(ServerError::PlayInAuction);
            },
            &mut Game::Playing(ref mut game) => {
                match game.play_card(info.pos, card) {
                    Ok(result) => result,
                    Err(err) => {
                        // Nothing to see here
                        return Err(ServerError::Play(err));
                    },
                }
            },
        };
        party.add_event(EventType::FromPlayer(info.pos, PlayerEvent::CardPlayed(card)));
        match result {
            game::TrickResult::Nothing => (),
            game::TrickResult::TrickOver(winner) => party.add_event(EventType::TrickOver(winner)),
        }

        // TODO: check for gameover?

        // Dummy event before handling the real case
        Ok(Event{
            event: EventType::BidCancelled,
            id:0
        })
    }

    // TODO: add bidding and stuff?


    // Waits until the given event_id happens
    pub fn wait(&self, player_id: u32, event_id: usize) -> Result<Event,ServerError> {
        let res = self.get_wait_result(player_id, event_id);

        match res {
            Err(err) => Err(err),
            Ok(WaitResult::Ready(event)) => Ok(event),
            // TODO: handle case where the wait is cancelled
            // (don't unwrap, return an error instead?)
            Ok(WaitResult::Waiting(rx)) => Ok(rx.recv().unwrap()),
        }
    }

    // Check if the event ID is already available. If not, returns a channel that will produce it one
    // day, so that we don't keep the locks while waiting.
    fn get_wait_result(&self, player_id: u32, event_id: usize) -> Result<WaitResult,ServerError> {
        let list = self.party_list.read().unwrap();
        if !list.player_map.contains_key(&player_id) {
            return Err(ServerError::BadPlayerId);
        }

        let info = list.player_map.get(&player_id).unwrap();
        let mut party = info.party.write().unwrap();

        if party.events.len() > event_id {
            return Ok(WaitResult::Ready(Event {
                event: party.events[event_id].clone(),
                id: event_id,
            }));
        } else if event_id > party.events.len() {
            // We are too ambitious! One event at a time!
            return Err(ServerError::BadEventId);
        }

        let (tx, rx) = mpsc::channel();
        party.observers.push(tx);

        Ok(WaitResult::Waiting(rx))
    }
}

