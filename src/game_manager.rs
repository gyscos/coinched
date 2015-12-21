use rand::{thread_rng,Rng};
use time;

use std::fmt;
use std::collections::HashMap;
use std::sync::{Arc,RwLock,Mutex,mpsc};
use std::convert::From;

use super::libcoinche::{bid,cards,pos,game,trick};

use rustc_serialize;

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
            &ManagerError::BadPlayerId => write!(f, "bad Player ID"),
            &ManagerError::BadEventId  => write!(f, "bad Event ID"),
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
    Bidded(bid::Contract),
    Coinched,
    Passed,
    CardPlayed(cards::Card),
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
    TrickOver(pos::PlayerPos),

    // New game: contains the first player, and the player's hand
    NewGame(pos::PlayerPos, [cards::Hand;4]),

    // Game over: contains scores
    GameOver([i32;2], pos::Team, [i32;2]),
}

// Ugly serialization...
impl rustc_serialize::Encodable for EventType {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        match self {
            &EventType::PartyCancelled(ref msg) => {
                s.emit_struct("Event", 2, |s| {
                    try!(s.emit_struct_field("type", 0, |s| "PartyCancelled".encode(s)));
                    try!(s.emit_struct_field("msg", 1, |s| msg.encode(s)));
                    Ok(())
                })
            },
            &EventType::BidCancelled => {
                s.emit_struct("Event", 1, |s| {
                    s.emit_struct_field("type", 0, |s| "BidCancelled".encode(s))
                })
            },
            &EventType::BidOver(ref contract) => {
                s.emit_struct("Event", 2, |s| {
                    try!(s.emit_struct_field("type", 0, |s| "BidOver".encode(s)));
                    try!(s.emit_struct_field("contract", 1, |s| contract.encode(s)));
                    Ok(())
                })
            },
            _ => Ok(())
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

    waiting_list: Mutex<Vec<mpsc::Sender<NewPartyInfo>>>,
}

/// Describe a single game.
pub enum Game {
    /// The game is still in the auction phase
    Bidding(bid::Auction),
    /// The game is in the main playing phase
    Playing(game::GameState),
}

fn make_game(first: pos::PlayerPos) -> (bid::Auction, EventType) {
    let auction = bid::new_auction(first);
    let hands = auction.hands();

    let event = EventType::NewGame(first, hands);

    (auction,event)
}

pub struct Party {
    game: Game,
    first: pos::PlayerPos,

    scores: [i32; 2],

    events: Vec<EventType>,
    observers: Mutex<Vec<mpsc::Sender<Event>>>,
}

fn new_party(first: pos::PlayerPos) -> Party {
    let (auction,event) = make_game(first);
    Party {
        first: first,
        game: Game::Bidding(auction),
        scores: [0;2],
        events: vec![event],
        observers: Mutex::new(Vec::new()),
    }
}

impl Party {
    fn add_event(&mut self, event: EventType) -> Event {
        let ev = Event{
            event: event.clone(),
            id: self.events.len(),
        };
        let mut observers = self.observers.lock().unwrap();
        for sender in observers.iter() {
            // TODO: handle cancelled wait?
            sender.send(ev.clone()).unwrap();
        }
        observers.clear();
        self.events.push(event);

        ev
    }

    fn get_auction_mut(&mut self) -> Result<&mut bid::Auction,ManagerError> {
        match self.game {
            Game::Bidding(ref mut auction) => Ok(auction),
            Game::Playing(_) => Err(ManagerError::BidInGame),
        }
    }

    fn get_game(&self) -> Result<&game::GameState,ManagerError> {
        match self.game {
            Game::Bidding(_) => Err(ManagerError::PlayInAuction),
            Game::Playing(ref game) => Ok(game),
        }
    }

    fn get_game_mut(&mut self) -> Result<&mut game::GameState,ManagerError> {
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

    fn bid(&mut self, pos: pos::PlayerPos, contract: bid::Contract) -> Result<Event,ManagerError> {
        let state = {
            let auction = try!(self.get_auction_mut());
            try!(auction.bid(contract.clone()))
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Bidded(contract.clone())));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            _ => (),
        }

        Ok(main_event)
    }

    fn pass(&mut self, pos: pos::PlayerPos) -> Result<Event,ManagerError> {
        let state = try!(self.get_auction_mut()).pass();

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
            try!(auction.coinche())
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
                self.add_event(EventType::TrickOver(winner));
                match game_result {
                    game::GameResult::Nothing => (),
                    game::GameResult::GameOver(points, winners, scores) => {
                        for i in 0..2 { self.scores[i] += scores[i]; }
                        let total_scores = self.scores;
                        self.add_event(EventType::GameOver(points, winners, total_scores));
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
    Waiting(mpsc::Receiver<Event>),
}

enum JoinResult {
    Ready(NewPartyInfo),
    Waiting(mpsc::Receiver<NewPartyInfo>),
}

impl GameManager {
    pub fn new() -> GameManager {
        GameManager {
            party_list: RwLock::new(PlayerList::new()),
            waiting_list: Mutex::new(Vec::new()),
        }
    }

    /// Attempts to join a new party. Blocks until a party is available.
    pub fn join(&self) -> Result<NewPartyInfo, String> {
        match self.get_join_result() {
            // TODO: add a timeout (max: 20s)
            // TODO: handle cancelled join?
            JoinResult::Ready(info) => Ok(info),
            JoinResult::Waiting(rx) => Ok(rx.recv().unwrap()),
        }
    }

    fn get_join_result(&self) -> JoinResult {
        let mut waiters = self.waiting_list.lock().unwrap();
        // println!("Waiters: {}", waiters.len());
        if waiters.len() >= 3 {
            // It's a PARTEY!
            let info = self.make_party([
                                       waiters.pop().unwrap(),
                                       waiters.pop().unwrap(),
                                       waiters.pop().unwrap(),
            ]);
            // println!("PARTEY INCOMING");
            return JoinResult::Ready(info);
        } else {
            let (tx,rx) = mpsc::channel();
            waiters.push(tx);
            return JoinResult::Waiting(rx);
        }
    }

    fn make_party(&self, others: [mpsc::Sender<NewPartyInfo>; 3]) -> NewPartyInfo {
        let mut list = self.party_list.write().unwrap();

        // println!("Making a party now!");

        // Generate 4 new IDS
        let ids = list.make_ids();

        // println!("IDS: {:?}", ids);

        let party = Arc::new(RwLock::new(new_party(pos::P0)));
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
        for i in 0..3 {
            others[i].send(NewPartyInfo{
                player_id: ids[i],
                player_pos: pos::PlayerPos(i),
            }).unwrap();
        }

        // println!("Almost ready!");

        // Even you, weird 4th dude.
        NewPartyInfo{
            player_id: ids[3],
            player_pos: pos::P3,
        }
    }

    // Play a card in the current game
    pub fn play_card(&self, player_id: u32, card: cards::Card) -> Result<Event,ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));


        let mut party = info.party.write().unwrap();
        party.play_card(info.pos, card)

    }

    pub fn bid(&self, player_id: u32, contract: bid::Contract) -> Result<Event,ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.bid(info.pos, contract)
    }

    pub fn pass(&self, player_id: u32) -> Result<Event, ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.pass(info.pos)
    }

    pub fn coinche(&self, player_id: u32) -> Result<Event, ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let mut party = info.party.write().unwrap();
        party.coinche(info.pos)
    }

    pub fn see_hand(&self, player_id: u32) -> Result<cards::Hand, ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let hands = match party.game {
            Game::Bidding(ref auction) => auction.hands(),
            Game::Playing(ref game) => game.hands(),
        };

        Ok(hands[info.pos.0])
    }

    pub fn see_trick(&self, player_id: u32) -> Result<trick::Trick,ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let game = try!(party.get_game());
        Ok(game.current_trick().clone())
    }

    pub fn see_last_trick(&self, player_id: u32) -> Result<trick::Trick, ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        let game = try!(party.get_game());
        let trick = try!(game.last_trick());
        Ok(trick.clone())
    }

    pub fn see_scores(&self, player_id: u32) -> Result<[i32;2],ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();
        Ok(party.scores)
    }

    pub fn see_pos(&self, player_id: u32) -> Result<pos::PlayerPos,ManagerError> {
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
    pub fn wait(&self, player_id: u32, event_id: usize) -> Result<Event,ManagerError> {
        let res = try!(self.get_wait_result(player_id, event_id));

        // TODO: add a timeout (~15s?)

        match res {
            WaitResult::Ready(event) => Ok(event),
            // TODO: handle case where the wait is cancelled
            // (don't unwrap, return an error instead?)
            WaitResult::Waiting(rx) => Ok(rx.recv().unwrap()),
        }
    }

    // Check if the event ID is already available. If not, returns a channel that will produce it one
    // day, so that we don't keep the locks while waiting.
    fn get_wait_result(&self, player_id: u32, event_id: usize) -> Result<WaitResult,ManagerError> {
        let list = self.party_list.read().unwrap();
        let info = try!(list.get_player_info(player_id));

        let party = info.party.read().unwrap();

        if party.events.len() > event_id {
            return Ok(WaitResult::Ready(Event {
                event: party.events[event_id].clone(),
                id: event_id,
            }));
        } else if event_id > party.events.len() {
            // We are too ambitious! One event at a time!
            return Err(ManagerError::BadEventId);
        }

        // Ok, so we'll have to wait a bit.

        let (tx, rx) = mpsc::channel();
        party.observers.lock().unwrap().push(tx);

        Ok(WaitResult::Waiting(rx))
    }
}

