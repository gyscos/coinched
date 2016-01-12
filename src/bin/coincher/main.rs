extern crate coinched;
extern crate libcoinche;
extern crate hyper;
extern crate rustc_serialize;
extern crate url;

mod client;
mod error;
mod auction;

use std::io;
use std::io::{BufRead};
use libcoinche::pos::PlayerPos;



fn card_play<F: FnMut() -> String>(client: &mut client::Client,
                                   first: PlayerPos,
                                   hand: libcoinche::cards::Hand,
                                   contract: libcoinche::bid::Contract,
                                   input: &mut F) -> Option<[i32; 2]> {
    None

}

fn play_game<F: FnMut() -> String>(client: &mut client::Client,
                                   first: PlayerPos,
                                   hand: libcoinche::cards::Hand,
                                   input: &mut F) -> Option<[i32; 2]> {
    print!("Cards:\n[");
    for card in hand.list() {
        print!(" {}", card.to_string());
    }
    println!(" ]");

    // Start with auction
    let contract = match auction::run_auction(client, input) {
        auction::AuctionResult::Complete(contract) => contract,
        auction::AuctionResult::Abort => return None,
        auction::AuctionResult::TryAgain => return Some([0, 0]),
    };

    // Continue with card play
    let scores = match card_play(client, first, hand, contract, input) {
        Some(scores) => scores,
        None => return None,
    };

    Some(scores)
}

fn main() {
    // TODO: read this from arguments
    let host = "localhost:3000";

    // TODO: allow reconnecting to an existing game

    let mut client = client::Client::join(host).unwrap();
    let mut score: [i32; 2] = [0, 0];

    // We'll receive input from stdin
    let mut stdin_reader = io::BufReader::new(io::stdin());
    let mut read_line = || {
        let mut buffer = String::new();
        stdin_reader.read_line(&mut buffer).unwrap();
        buffer.pop().unwrap();
        buffer
    };

    // Keep playing until someone leaves.
    loop {
        let event = client.wait().unwrap();
        match event {
            // A game started! Let's finish it.
            coinched::EventType::NewGameRelative{first, hand} => {
                let new_scores = match play_game(&mut client, first, hand, &mut read_line) {
                    Some(scores) => scores,
                    None => return,
                };
                score[0] += new_scores[0];
                score[1] += new_scores[1];
            }
            // Outside of a game, we don't really expect any other event.
            // TODO: PartyCancelled maybe?
            _ => panic!("Unexpected event: {:?}", event),
        }
    }
}
