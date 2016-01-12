use std::io;
use std::io::Write;
use coinched::{EventType, PlayerEvent};
use libcoinche::{bid, cards};
use std::str::FromStr;

use client::Client;

/// Runs an auction and returns the result
pub fn run_auction<F: FnMut() -> String>(client: &mut Client, input: &mut F) -> AuctionResult {
    Auction::new(client).run(input)
}

/// What could result from an auction?
pub enum AuctionResult {
    /// Auction is complete: here's the contract
    Complete(bid::Contract),
    /// Auction didn't lead to anything, let's try again
    TryAgain,
    /// Someone left! Abort! Abort!
    Abort,
}

struct Auction<'a> {
    client: &'a mut Client,
}

enum AuctionAction {
    Leave,
    Pass,
    Coinche,
    Bid((bid::Target, cards::Suit)),
}

fn parse_bid(line: &str) -> Result<(bid::Target, cards::Suit), String> {
    let tokens: Vec<&str> = line.trim().split(" ").collect();
    if tokens.len() != 2 {
        return Err("Invalid number of tokens".to_string());
    }

    let target = try!(bid::Target::from_str(tokens[0]));
    let suit = try!(cards::Suit::from_str(tokens[1]));

    Ok((target, suit))
}


impl <'a> Auction<'a> {
    fn new(client: &'a mut Client) -> Self {
        Auction {
            client: client,
        }
    }

    fn read_input<F: FnMut() -> String>(&self, input: &mut F) -> AuctionAction {
        loop {
            println!("Your turn to bid. Commands:");
            println!("* `leave`");
            println!("* `pass`");
            println!("* `coinche`");
            println!("* [80, 90, ... , Capot] [H,C,D,S]");
            print!("> ");
            io::stdout().flush().unwrap();

            let line = input();

            return match line.as_ref() {
                // Those are easy actions
                "leave" => AuctionAction::Leave,
                "pass" => AuctionAction::Pass,
                "coinche" => AuctionAction::Coinche,
                line => {
                    // Here we parse the bid
                    let contract = match parse_bid(line) {
                        Err(msg) => {
                            println!("{}", msg);
                            continue;
                        },
                        Ok(contract) => contract,
                    };

                    AuctionAction::Bid(contract)
                }
            };
        }
    }

    fn run<F: FnMut() -> String>(mut self, input: &mut F) -> AuctionResult {

        loop {
            let mut event = self.client.wait();
            match event {
                Ok(EventType::YourTurn) => event = match self.read_input(input) {
                    AuctionAction::Leave => return AuctionResult::Abort,
                    AuctionAction::Coinche => self.client.coinche(),
                    AuctionAction::Pass => self.client.pass(),
                    AuctionAction::Bid(contract) => self.client.bid(contract),
                },
                _ => (),
            }

            match event {
                Ok(EventType::FromPlayer(pos, e)) => {
                    match e {
                        PlayerEvent::Bidded(trump, target) =>
                            println!("Player {:?} bidded {:?} on {:?}.", pos, target, trump),
                            PlayerEvent::Passed | PlayerEvent::Coinched =>
                                println!("Player {:?} passed.", pos),
                                e => println!("Unexpected event: {:?}", e),
                    }
                }
                Ok(EventType::BidCancelled) => {
                    return AuctionResult::TryAgain;
                },
                Ok(EventType::PartyCancelled(msg)) => {
                    println!("party cancelled: {}", msg);
                    return AuctionResult::Abort;
                },
                Ok(EventType::BidOver(contract)) => {
                    return AuctionResult::Complete(contract);
                },
                Ok(event) => println!("Unexpected event received: {:?}", event),
                Err(err) => println!("Error: {:?}", err),
            }
        }
    }
}
