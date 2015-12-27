use rand::{thread_rng,Rng};
use time;

use std::fmt;
use std::collections::HashMap;
use std::sync::{Arc,RwLock,Mutex};
use std::convert::From;

use super::libcoinche::{bid,cards,pos,game,trick};

use rustc_serialize;
use eventual::{Future,Complete,Async};

pub type ManagerResult <T> = Result<T, ManagerError>;

pub enum ManagerError {
    BadPlayerId,
    BadEventId,

    PlayInAuction,
    BidInGame,

    Bid(bid::BidError),
    Play(game::PlayError),
}

impl fmt::Display for ManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &ManagerError::BadPlayerId => write!(f, "player not found"),
            &ManagerError::BadEventId  => write!(f, "event not found"),
            &ManagerError::PlayInAuction => write!(f, "cannot play during auction"),
            &ManagerError::BidInGame => write!(f, "cannot bid during card play"),
            &ManagerError::Bid(ref error) => write!(f, "{}", error),
            &ManagerError::Play(ref error) => write!(f, "{}", error),
        }
    }
}

impl From<bid::BidError> for ManagerError {
    fn from(err: bid::BidError) -> ManagerError {
        ManagerError::Bid(err)
    }
}
impl From<game::PlayError> for ManagerError {
    fn from(err: game::PlayError) -> ManagerError {
        ManagerError::Play(err)
    }
}

pub enum Action {
    Bid(bid::Contract),
    Coinche,
    Pass,
    Play(cards::Card),
}

#[derive(Clone)]
pub enum PlayerEvent {
    Bidded(cards::Suit, bid::Target),
    Coinched,
    Passed,
    CardPlayed(cards::Card),
}

impl rustc_serialize::Encodable for PlayerEvent {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self {
            &PlayerEvent::Bidded(suit, target) => s.emit_struct("PlayerEvent", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "Bidded".encode(s)));
                try!(s.emit_struct_field("suit", 1, |s| suit.encode(s)));
                try!(s.emit_struct_field("target", 2, |s| target.encode(s)));
                Ok(())
            }),
            &PlayerEvent::Coinched => s.emit_struct("PlayerEvent", 1, |s| {
                s.emit_struct_field("type", 0, |s| "Coinched".encode(s))
            }),
            &PlayerEvent::Passed => s.emit_struct("PlayerEvent", 1, |s| {
                s.emit_struct_field("type", 0, |s| "Passed".encode(s))
            }),
            &PlayerEvent::CardPlayed(card) => s.emit_struct("PlayerEvent", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "CardPlayed".encode(s)));
                try!(s.emit_struct_field("card", 1, |s| card.encode(s)));
                Ok(())
            }),
        }
    }
}

// Player just joined a new party. He's given a player id, and his position.
#[derive(RustcEncodable)]
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
    BidOver(bid::Contract),
    // The bid was cancelled, probably because no one bidded anything.
    // A new game is probably on its way.
    BidCancelled,

    // Trick over: contains the winner
    TrickOver { winner: pos::PlayerPos },

    // New game: contains the first player, and the player's hand
    NewGame { first: pos::PlayerPos, hands: [cards::Hand;4] },
    NewGameRelative { first: pos::PlayerPos, hand: cards::Hand },

    // Game over: contains scores
    GameOver { points: [i32;2], winner: pos::Team, scores: [i32;2] },
}

impl EventType {
    pub fn relativize(&self, from: pos::PlayerPos) -> Self {
        match self {
            &EventType::NewGame { first, hands } =>
                EventType::NewGameRelative { first: first, hand: hands[from.0] },
            _ => self.clone(),
        }
    }
}

// Ugly serialization...
impl rustc_serialize::Encodable for EventType {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self {
            &EventType::PartyCancelled(ref msg) => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "PartyCancelled".encode(s)));
                try!(s.emit_struct_field("msg", 1, |s| msg.encode(s)));
                Ok(())
            }),
            &EventType::BidCancelled => s.emit_struct("Event", 1, |s| {
                s.emit_struct_field("type", 0, |s| "BidCancelled".encode(s))
            }),
            &EventType::BidOver(ref contract) => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "BidOver".encode(s)));
                try!(s.emit_struct_field("contract", 1, |s| contract.encode(s)));
                Ok(())
            }),
            &EventType::TrickOver { winner: pos } => s.emit_struct("Event", 2, |s| {
                try!(s.emit_struct_field("type", 0, |s| "TrickOver".encode(s)));
                try!(s.emit_struct_field("pos", 1, |s| pos.encode(s)));
                Ok(())
            }),
            &EventType::FromPlayer(pos, ref event) => s.emit_struct("Event", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "FromPlayer".encode(s)));
                try!(s.emit_struct_field("pos", 1, |s| pos.encode(s)));
                try!(s.emit_struct_field("event", 2, |s| event.encode(s)));
                Ok(())
            }),
            &EventType::NewGame { first, ref hands } => s.emit_struct("Event", 3, |s| {
                // Should rarely happen
                try!(s.emit_struct_field("type", 0, |s| "NewGameGlobal".encode(s)));
                try!(s.emit_struct_field("first", 1, |s| first.encode(s)));
                try!(s.emit_struct_field("hands", 2, |s| hands.encode(s)));
                Ok(())
            }),
            &EventType::NewGameRelative { first, ref hand } => s.emit_struct("Event", 3, |s| {
                try!(s.emit_struct_field("type", 0, |s| "NewGame".encode(s)));
                try!(s.emit_struct_field("first", 1, |s| first.encode(s)));
                try!(s.emit_struct_field("cards", 2, |s| hand.encode(s)));
                Ok(())
            }),
            &EventType::GameOver { points, winner, scores } => s.emit_struct("Event", 4, |s| {
                try!(s.emit_struct_field("type", 0, |s| "GameOver".encode(s)));
                try!(s.emit_struct_field("points", 1, |s| points.encode(s)));
                try!(s.emit_struct_field("winner", 2, |s| winner.encode(s)));
                try!(s.emit_struct_field("scores", 3, |s| scores.encode(s)));
                Ok(())
            }),
        }
    }
}

#[derive(Clone,RustcEncodable)]
pub struct Event {
    pub event: EventType,
    pub id: usize,
}

pub struct Order {
    pub author: pos::PlayerPos,
    pub action: Action
}

/// Base class for managing matchmaking.
///
/// It is the main entry point for the API.
/// It offers a thread-safe access to various actions.
pub struct GameManager {
    party_list: RwLock<PlayerList>,

    waiting_list: Mutex<Vec<Complete<NewPartyInfo,()>>>,
}

/// Describe a single game.
pub enum Game {
    /// The game is still in the auction phase
    Bidding(bid::Auction),
    /// The game is in the main playing phase
    Playing(game::GameState),
}

fn make_game(first: pos::PlayerPos) -> (bid::Auction, EventType) {
    let auction = bid::Auction::new(first);
    let hands = auction.hands();

    let event = EventType::NewGame { first: first, hands: hands };

    (auction,event)
}

pub struct Party {
    game: Game,
    first: pos::PlayerPos,

    scores: [i32; 2],

    events: Vec<EventType>,
    observers: Mutex<Vec<Complete<Event,()>>>,
}

impl Party {
    fn new(first: pos::PlayerPos) -> Self {
        let (auction,event) = make_game(first);
        Party {
            first: first,
            game: Game::Bidding(auction),
            scores: [0;2],
            events: vec![event],
            observers: Mutex::new(Vec::new()),
        }
    }

    fn add_event(&mut self, event: EventType) -> Event {
        let ev = Event{
            event: event.clone(),
            id: self.events.len(),
        };
        let mut observers = self.observers.lock().unwrap();
        for promise in observers.drain(..) {
            // TODO: handle cancelled wait?
            promise.complete(ev.clone());
        }
        self.events.push(event);

        ev
    }

    fn get_auction_mut(&mut self) -> ManagerResult<&mut bid::Auction> {
        match self.game {
            Game::Bidding(ref mut auction) => Ok(auction),
            Game::Playing(_) => Err(ManagerError::BidInGame),
        }
    }

    fn get_game(&self) -> ManagerResult<&game::GameState> {
        match self.game {
            Game::Bidding(_) => Err(ManagerError::PlayInAuction),
            Game::Playing(ref game) => Ok(game),
        }
    }

    fn get_game_mut(&mut self) -> ManagerResult<&mut game::GameState> {
        match self.game {
            Game::Bidding(_) => Err(ManagerError::PlayInAuction),
            Game::Playing(ref mut game) => Ok(game),
        }
    }

    fn next_game(&mut self) {
        // TODO: Maybe keep the current game in the history?

        let (auction, event) = make_game(self.first);

        self.first = self.first.next();
        self.game = Game::Bidding(auction);
        self.add_event(event);
    }

    fn cancel(&mut self) {
        self.add_event(EventType::PartyCancelled("player left".to_string()));
    }

    fn bid(&mut self, pos: pos::PlayerPos, trump: cards::Suit, target: bid::Target) -> ManagerResult<Event> {
        let state = {
            let auction = try!(self.get_auction_mut());
            try!(auction.bid(pos, trump, target))
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Bidded(trump, target)));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            _ => (),
        }

        Ok(main_event)
    }

    fn pass(&mut self, pos: pos::PlayerPos) -> Result<Event,ManagerError> {
        let state = {
            let auction = try!(self.get_auction_mut());
            try!(auction.pass(pos))
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Passed));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            bid::AuctionState::Cancelled => {
                self.add_event(EventType::BidCancelled);
                self.next_game();
            },
            _ => (),
        }

        Ok(main_event)
    }

    fn coinche(&mut self, pos: pos::PlayerPos) -> Result<Event, ManagerError> {
        let state = {
            let auction = try!(self.get_auction_mut());
            try!(auction.coinche(pos))
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Coinched));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            _ => (),
        }

        Ok(main_event)
    }

    fn complete_auction(&mut self) {
        let game = match &mut self.game {
            &mut Game::Playing(_) => unreachable!(),
            &mut Game::Bidding(ref mut auction) => {
                match auction.complete() {
                    Ok(game) => game,
                    Err(err) => panic!(err),
                }
            }
        };

        self.add_event(EventType::BidOver(game.contract().clone()));

        self.game = Game::Playing(game);
    }

    fn play_card(&mut self, pos: pos::PlayerPos, card: cards::Card) -> Result<Event,ManagerError> {
        let result = {
            let game = try!(self.get_game_mut());
            try!(game.play_card(pos, card))
        };

        // This is the main event we want to send.
        // TODO: Batch event dispatch, and send all those together.
        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::CardPlayed(card)));
        match result {
            game::TrickResult::Nothing => (),
            game::TrickResult::TrickOver(winner, game_result) => {
                self.add_event(EventType::TrickOver{ winner: winner });
                match game_result {
                    game::GameResult::Nothing => (),
                    game::GameResult::GameOver{points, winners, scores} => {
                        for i in 0..2 { self.scores[i] += scores[i]; }
                        self.add_event(EventType::GameOver {
                            points: points,
                            winner: winners,
                            scores: scores,
                        });
                        self.next_game();
                    }
                }
            },
        }

        Ok(main_event)
    }
}

pub struct PlayerInfo {
    pub party: Arc<RwLock<Party>>,
    pub pos: pos::PlayerPos,
    pub last_time: Mutex<time::Tm>,
}

pub struct PlayerList {
    pub player_map: HashMap<u32,PlayerInfo>,
}

impl PlayerList {
    fn new() -> PlayerList {
        PlayerList {
            player_map: HashMap::new(),
        }
    }

    fn get_player_info(&self, player_id: u32) -> Result<&PlayerInfo,ManagerError> {
        match self.player_map.get(&player_id) {
            None => Err(ManagerError::BadPlayerId),
            Some(info) => {
                // Update the last active time
                *info.last_time.lock().unwrap() = time::now();
                Ok(info)
            },
        }
    }

    fn make_ids(&self) -> [u32; 4] {
        // Expect self.player_map to be locked
        let mut result = [0;4];

        for i in 0..4 {
            loop {
                let id = thread_rng().next_u32();
                // println!("New UUID: {}", id);
                if self.player_map.contains_key(&id) {
                    // println!("Damnation!");
                    continue;
                }
                let mut ok = true;
                for j in 0..i {
                    if result[j] == id {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    result[i] = id;
                    break;
                }
            }
        }

        result
    }

    fn remove(&mut self, player_id: u32) {
        self.player_map.get(&player_id).unwrap().party.write().unwrap().cancel();
        self.player_map.remove(&player_id);
    }
}

enum WaitResult {
    Ready(Event),
    Waiting(Future<Event,()>),
}

enum JoinResult {
    Ready(NewPartyInfo),
    Waiting(Future<NewPartyInfo,()>),
}

impl GameManager {
    pub fn new() -> GameManager {
        GameManager {
            party_list: RwLock::new(PlayerList::new()),
            waiting_list: Mutex::new(Vec::new()),
        }
    }

    /// Attempts to join a new party. Blocks until a party is available.
    pub fn join(&self) -> ManagerResult<NewPartyInfo> {
        match self.get_join_result() {
            // TODO: add a timeout (max: 20s)
            // TODO: handle cancelled join?
            JoinResult::Ready(info) => Ok(info),
            JoinResult::Waiting(future) => Ok(future.await().unwrap()),
        }
    }

    fn get_join_result(&self) -> JoinResult {
        let mut waiters = self.waiting_list.lock().unwrap();
        // println!("Waiters: {}", waiters.len());
        if waiters.len() >= 3 {
            // It's a PARTEY!
            let info = self.make_party(vec![
               waiters.pop().unwrap(),
               waiters.pop().unwrap(),
               waiters.pop().unwrap(),
            ]);
            // println!("PARTEY INCOMING");
            return JoinResult::Ready(info);
        } else {
            let (promise,future) = Future::pair();
            waiters.push(promise);
            return JoinResult::Waiting(future);
        }
    }

    fn make_party(&self, others: Vec<Complete<NewPartyInfo,()>>) -> NewPartyInfo {
        let mut list = self.party_list.write().unwrap();

        // println!("Making a party now!");

        // Generate 4 new IDS
        let ids = list.make_ids();

        // println!("IDS: {:?}", ids);

        let party = Arc::new(RwLock::new(Party::new(pos::P0)));
        // Kickstart it with a new game!

        // Prepare the players info
        for i in 0..4 {
            list.player_map.insert(ids[i], PlayerInfo {
                party: party.clone(),
                pos: pos::PlayerPos(i),
                last_time: Mutex::new(time::now()),
            });
        }

        // Tell everyone. They'll love it.
        // TODO: handle cancelled channels (?)
        // println!("Waking them up!");
        for (i,promise) in others.into_iter().enumerate() {
            promise.complete(NewPartyInfo {
                player_id: ids[i],
                player_pos: pos::PlayerPos(i),
            });
        }

        // println!("Almost ready!");

        // Even you, weird 4th dude.
        NewPartyInfo{
            player_id: ids[3],
            player_pos: pos::P3,
        }
    }

    // Play a card in the current game
    pub fn play_card(&self, player_id: u32, card: cards::Card) -> ManagerResult<Event> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));


        let mut party = info.party.write().unwrap();
        party.play_card(info.pos, card)

    }

    pub fn bid(&self, player_id: u32, (target, trump): (bid::Target, cards::Suit)) -> ManagerResult<Event> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.bid(info.pos, trump, target)
    }

    pub fn pass(&self, player_id: u32) -> ManagerResult<Event> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.pass(info.pos)
    }

    pub fn coinche(&self, player_id: u32) -> ManagerResult<Event> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.coinche(info.pos)
    }

    pub fn see_hand(&self, player_id: u32) -> ManagerResult<cards::Hand> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let hands = match party.game {
            Game::Bidding(ref auction) => auction.hands(),
            Game::Playing(ref game) => game.hands(),
        };

        Ok(hands[info.pos.0])
    }

    pub fn see_trick(&self, player_id: u32) -> ManagerResult<trick::Trick> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let game = try!(party.get_game());
        Ok(game.current_trick().clone())
    }

    pub fn see_last_trick(&self, player_id: u32) -> ManagerResult<trick::Trick> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let game = try!(party.get_game());
        let trick = try!(game.last_trick());
        Ok(trick.clone())
    }

    pub fn see_scores(&self, player_id: u32) -> ManagerResult<[i32;2]> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        Ok(party.scores)
    }

    pub fn see_pos(&self, player_id: u32) -> ManagerResult<pos::PlayerPos> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));
        Ok(info.pos)
    }

    // TODO: auto-leave players after long inactivity
    pub fn leave(&self, player_id: u32) {
        let mut list = self.party_list.write().unwrap();

        list.remove(player_id);
    }

    // Waits until the given event_id happens
    pub fn wait(&self, player_id: u32, event_id: usize) -> ManagerResult<Event> {
        let res = try!(self.get_wait_result(player_id, event_id));

        // TODO: add a timeout (~15s?)

        match res {
            WaitResult::Ready(event) => Ok(event),
            // TODO: handle case where the wait is cancelled
            // (don't unwrap, return an error instead?)
            WaitResult::Waiting(future) => Ok(future.await().unwrap()),
        }
    }

    // Check if the event ID is already available. If not, returns a channel that will produce it one
    // day, so that we don't keep the locks while waiting.
    fn get_wait_result(&self, player_id: u32, event_id: usize) -> ManagerResult<WaitResult> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();

        if party.events.len() > event_id {
            return Ok(WaitResult::Ready(Event {
                event: party.events[event_id].relativize(info.pos),
                id: event_id,
            }));
        } else if event_id > party.events.len() {
            // We are too ambitious! One event at a time!
            return Err(ManagerError::BadEventId);
        }

        // Ok, so we'll have to wait a bit.

        let (promise, future) = Future::pair();
        party.observers.lock().unwrap().push(promise);

        Ok(WaitResult::Waiting(future))
    }
}

