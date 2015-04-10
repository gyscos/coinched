extern crate rand;

use self::rand::{thread_rng,Rng};

use std::collections::HashMap;
use std::sync::{Arc,RwLock,Mutex,mpsc};

use super::libcoinche::{bid,cards,pos,game,trick};

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

    fn new_game(&mut self) {
        // TODO: Maybe keep the current game in the history?

        let (auction, event) = make_game(self.first);

        self.first = self.first.next();
        self.game = Game::Bidding(auction);
        self.add_event(event);
    }

    fn cancel(&mut self) {
        self.add_event(EventType::PartyCancelled("player left".to_string()));
    }

    fn bid(&mut self, pos: pos::PlayerPos, contract: bid::Contract) -> Result<Event,ServerError> {
        let state = match &mut self.game {
            &mut Game::Bidding(ref mut auction) => {
                match auction.bid(contract.clone()) {
                    Ok(state) => state,
                    Err(err) => return Err(ServerError::Bid(err)),
                }

            },
            &mut Game::Playing(_) => return Err(ServerError::BidInGame),
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Bidded(contract.clone())));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            _ => (),
        }

        Ok(main_event)
    }

    fn pass(&mut self, pos: pos::PlayerPos) -> Result<Event,ServerError> {
        let state = match &mut self.game {
            &mut Game::Bidding(ref mut auction) => auction.pass(),
            &mut Game::Playing(_) => return Err(ServerError::BidInGame),
        };

        let main_event = self.add_event(EventType::FromPlayer(pos, PlayerEvent::Passed));
        match state {
            bid::AuctionState::Over => self.complete_auction(),
            bid::AuctionState::Cancelled => {
                self.add_event(EventType::BidCancelled);
                self.new_game();
            },
            _ => (),
        }

        Ok(main_event)
    }

    fn coinche(&mut self, pos: pos::PlayerPos) -> Result<Event, ServerError> {
        let state = match &mut self.game {
            &mut Game::Bidding(ref mut auction) => match auction.coinche() {
                Ok(state) => state,
                Err(err) => return Err(ServerError::Bid(err)),
            },
            &mut Game::Playing(_) => return Err(ServerError::BidInGame),
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

    fn play_card(&mut self, pos: pos::PlayerPos, card: cards::Card) -> Result<Event,ServerError> {
        let result = match &mut self.game {
            &mut Game::Playing(ref mut game) => {
                match game.play_card(pos, card) {
                    Ok(result) => result,
                    Err(err) => return Err(ServerError::Play(err)),
                }
            },
            &mut Game::Bidding(_) => {
                // Bad time for a card play!
                return Err(ServerError::PlayInAuction);
            },
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
                        self.new_game();
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

impl Server {
    pub fn join(&self) -> Option<NewPartyInfo> {
        match self.get_join_result() {
            // TODO: add a timeout (max: 20s)
            // TODO: handle cancelled join?
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
        // TODO: handle cancelled channels (?)
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

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let mut party = info.party.write().unwrap();
        party.play_card(info.pos, card)

    }

    pub fn bid(&self, player_id: u32, contract: bid::Contract) -> Result<Event,ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let mut party = info.party.write().unwrap();
        party.bid(info.pos, contract)
    }

    pub fn pass(&self, player_id: u32) -> Result<Event, ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let mut party = info.party.write().unwrap();
        party.pass(info.pos)
    }

    pub fn coinche(&self, player_id: u32) -> Result<Event, ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let mut party = info.party.write().unwrap();
        party.coinche(info.pos)
    }

    pub fn see_hand(&self, player_id: u32) -> Result<cards::Hand, ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let party = info.party.read().unwrap();
        let hands = match party.game {
            Game::Bidding(ref auction) => auction.hands(),
            Game::Playing(ref game) => game.hands(),
        };

        Ok(hands[info.pos.0])
    }

    pub fn see_trick(&self, player_id: u32) -> Result<trick::Trick,ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let party = info.party.read().unwrap();
        match party.game {
            Game::Bidding(_) => Err(ServerError::PlayInAuction),
            Game::Playing(ref game) => Ok(game.current_trick().clone()),
        }
    }

    pub fn see_last_trick(&self, player_id: u32) -> Result<trick::Trick, ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let party = info.party.read().unwrap();
        match party.game {
            Game::Bidding(_) => Err(ServerError::PlayInAuction),
            Game::Playing(ref game) => match game.last_trick() {
                Ok(trick) => Ok(trick.clone()),
                Err(err) => Err(ServerError::Play(err)),
            }
        }
    }

    pub fn see_scores(&self, player_id: u32) -> Result<[i32;2],ServerError> {
        let list = self.party_list.read().unwrap();

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let party = info.party.read().unwrap();
        Ok(party.scores)
    }

    pub fn see_pos(&self, player_id: u32) -> Result<pos::PlayerPos,ServerError> {
        let list = self.party_list.read().unwrap();

        match list.player_map.get(&player_id) {
            Some(info) => Ok(info.pos),
            None => Err(ServerError::BadPlayerId),
        }
    }

    // TODO: auto-leave players after long inactivity
    pub fn leave(&self, player_id: u32) {
        let mut list = self.party_list.write().unwrap();

        list.remove(player_id);
    }

    // Waits until the given event_id happens
    pub fn wait(&self, player_id: u32, event_id: usize) -> Result<Event,ServerError> {
        let res = self.get_wait_result(player_id, event_id);

        // TODO: add a timeout (~15s?)

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

        let info = match list.player_map.get(&player_id) {
            Some(info) => info,
            None => return Err(ServerError::BadPlayerId),
        };

        let party = info.party.read().unwrap();

        if party.events.len() > event_id {
            return Ok(WaitResult::Ready(Event {
                event: party.events[event_id].clone(),
                id: event_id,
            }));
        } else if event_id > party.events.len() {
            // We are too ambitious! One event at a time!
            return Err(ServerError::BadEventId);
        }

        // Ok, so we'll have to wait a bit.

        let (tx, rx) = mpsc::channel();
        party.observers.lock().unwrap().push(tx);

        Ok(WaitResult::Waiting(rx))
    }
}

