extern crate coinched;
extern crate libcoinche;

use std::io;
use std::io::{BufRead, Write};
use std::str::FromStr;
use libcoinche::{bid, cards, pos};
use coinched::EventType;
use coinched::client;

#[derive(Clone)]
struct CliFrontend;

fn parse_bid(line: &str) -> Result<(cards::Suit, bid::Target), String> {
    let tokens: Vec<&str> = line.trim().split(" ").collect();
    if tokens.len() != 2 {
        return Err("Invalid number of tokens".to_string());
    }

    let target = try!(bid::Target::from_str(tokens[0]));
    let suit = try!(cards::Suit::from_str(tokens[1]));

    Ok((suit, target))
}

impl CliFrontend {
    fn input() -> String {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        // Discard the `\n` at the end
        buffer.pop().unwrap();
        buffer
    }
}

impl client::Frontend<client::http::HttpBackend> for CliFrontend {
    fn show_error(&mut self, error: client::http::Error) {
        println!("Error: {:?}", error);
    }

    fn unexpected_event(&mut self, event: EventType) {
        println!("Unexpected event: {:?}", event);
    }

    fn party_cancelled(self, msg: String) {
        println!("Party cancelled: {}", msg);
    }
}

impl client::GameFrontend<client::http::HttpBackend> for CliFrontend {}

impl client::AuctionFrontend<client::http::HttpBackend> for CliFrontend {
    type Game = CliFrontend;

    fn show_pass(&mut self, pos: pos::PlayerPos) {
        println!("Player {:?} passed", pos);
    }

    fn show_coinche(&mut self, pos: pos::PlayerPos) {
        println!("Player {:?} coinched", pos);
    }

    fn show_bid(&mut self, pos: pos::PlayerPos, suit: cards::Suit, target: bid::Target) {
        println!("Player {:?} bid {} on {}",
                 pos,
                 target.to_string(),
                 suit.to_string());
    }

    fn ask_action(&mut self) -> client::AuctionAction {
        loop {
            println!("Your turn to bid. Commands:");
            println!("* `leave`");
            println!("* `pass`");
            println!("* `coinche`");
            println!("* [80, 90, ... , Capot] [H,C,D,S]");
            print!("> ");
            io::stdout().flush().unwrap();

            let line = Self::input();

            return match line.as_ref() {
                // Those are easy actions
                "leave" => client::AuctionAction::Leave,
                "pass" => client::AuctionAction::Pass,
                "coinche" => client::AuctionAction::Coinche,
                line => {
                    // Here we parse the bid
                    let contract = match parse_bid(line) {
                        Err(msg) => {
                            println!("{}", msg);
                            continue;
                        }
                        Ok(contract) => contract,
                    };

                    client::AuctionAction::Bid(contract)
                }
            };
        }
    }

    /// Auction cancelled, back to the start.
    fn auction_cancelled(self) {
        println!("Auction cancelled!");
    }

    /// Auction is complete, we can play now!
    fn auction_over(self, contract: &bid::Contract) -> Self::Game {
        println!("Auction is over: {:?}", contract);
        self
    }
}

impl client::MainFrontend<client::http::HttpBackend> for CliFrontend {
    type Auction = CliFrontend;

    fn start_game(&mut self, first: pos::PlayerPos, hand: cards::Hand) -> Self::Auction {
        print!("Cards:\n[");
        for card in hand.list() {
            print!(" {}", card.to_string());
        }
        println!(" ]");

        println!("First player: {:?}", first);

        self.clone()
    }
}

fn main() {
    // TODO: read this from arguments
    let host = "localhost:3000";

    // TODO: allow reconnecting to an existing game

    let backend = client::http::HttpBackend::join(host).unwrap();
    let frontend = CliFrontend;

    println!("{:?}", client::Client::new(backend, frontend).run());
}
