extern crate coinched;
extern crate libcoinche;
extern crate clap;

use std::io;
use std::io::{BufRead, Write};
use std::str::FromStr;
use libcoinche::{bid, cards, pos};
use coinched::EventType;
use coinched::client;
use clap::{Arg, App};

struct CliFrontend {
    hand: cards::Hand,
    pos: pos::PlayerPos,
}

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
    fn new(pos: pos::PlayerPos) -> Self {
        CliFrontend {
            pos: pos,
            hand: cards::Hand::new(),
        }
    }

    fn input() -> String {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        // Discard the `\n` at the end
        buffer.pop().unwrap();
        buffer
    }

    fn print_hand(&self) {
        print!("Cards: [");
        let cards = self.hand.list();
        let len = cards.len();
        for card in cards {
            print!(" {}", card.to_string());
        }
        println!(" ]");
        print!("        ");
        for i in 0..len {
            print!("  {}", i);
        }
        println!("");
    }
}

impl client::Frontend<client::http::HttpBackend> for CliFrontend {
    fn show_error(&mut self, error: client::http::Error) {
        println!("Error: {:?}", error);
    }

    fn unexpected_event(&mut self, event: EventType) {
        println!("Unexpected event: {:?}", event);
    }

    fn party_cancelled(&mut self, msg: &str) {
        println!("Party cancelled: {}", msg);
    }

    fn show_card_played(&mut self, pos: pos::PlayerPos, card: cards::Card) {
        println!("Player {:?} played {}", pos, card.to_string());
        if pos == self.pos {
            self.hand.remove(card);
        }
    }

    fn show_trick_over(&mut self, winner: pos::PlayerPos) {
        println!("{:?} gets the trick.", winner);
    }

    fn ask_card(&mut self) -> client::GameAction {
        let cards = self.hand.list();

        loop {
            self.print_hand();
            print!("What card do you play?\n> ");
            io::stdout().flush().unwrap();

            let line = Self::input();

            if line == "leave" {
                return client::GameAction::Leave;
            } else {
                match usize::from_str(&line) {
                    Ok(i) if i < cards.len() => return client::GameAction::PlayCard(cards[i]),
                    _ => println!("Invalid input."),
                }
            }
        }
    }

    fn game_over(&mut self, points: [i32; 2], winner: pos::Team, scores: [i32; 2]) {
        println!("Game over!");
        println!("{:?} won. Points were {:?} ; scores: {:?}",
                 winner,
                 points,
                 scores);
    }

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

    fn ask_bid(&mut self) -> client::AuctionAction {
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
    fn auction_cancelled(&mut self) {
        println!("Auction cancelled!");
    }

    /// Auction is complete, we can play now!
    fn auction_over(&mut self, contract: &bid::Contract) {
        println!("Auction is over: {:?}", contract);
    }

    fn start_game(&mut self, first: pos::PlayerPos, hand: cards::Hand) {
        self.hand = hand;

        self.print_hand();


        println!("First player: {:?}", first);
    }
}

fn main() {
    let matches = App::new("coincher")
                      .version(env!("CARGO_PKG_VERSION"))
                      .author("Alexandre Bury <alexandre.bury@gmail.com>")
                      .about("A client for coinched")
                      .arg(Arg::with_name("HOST")
                               .help("Specifies the host to connect to")
                               .required(true)
                               .index(1))
                      .get_matches();
    let host = matches.value_of("HOST").unwrap();

    // TODO: allow reconnecting to an existing game

    let backend = client::http::HttpBackend::join(host).unwrap();
    let mut frontend = CliFrontend::new(backend.pos);

    println!("Final score: {:?}",
             client::Client::new(backend).run(&mut frontend));
}
