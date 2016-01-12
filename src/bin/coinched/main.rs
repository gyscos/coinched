extern crate rustc_serialize;
extern crate rand;
extern crate time;
extern crate eventual;
extern crate coinched;
extern crate libcoinche;
extern crate iron;
extern crate bodyparser;
#[macro_use]
extern crate log;
extern crate env_logger;

mod error;
mod game_manager;
mod server;

fn main() {
    // TODO: read this from arguments
    env_logger::init().unwrap();

    let port = 3000;

    let server = server::Server::new(port);

    server.run();
}
