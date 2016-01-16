extern crate coinched;
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;

use std::str::FromStr;
use clap::{Arg, App};

fn main() {
    env_logger::init().unwrap();

    let matches = App::new("coinched")
                      .version(env!("CARGO_PKG_VERSION"))
                      .author("Alexandre Bury <alexandre.bury@gmail.com>")
                      .about("A coinche server")
                      .arg(Arg::with_name("PORT")
                               .help("Port to listen to (defaults to 3000)")
                               .short("p")
                               .long("port")
                               .takes_value(true))
                      .get_matches();

    let port = if let Some(port) = matches.value_of("PORT") {
        match u16::from_str(port) {
            Ok(port) => port,
            Err(err) => {
                println!("Invalid port: `{}` ({})", port, err);
                std::process::exit(1);
            }
        }
    } else {
        3000
    };

    let server = coinched::server::http::Server::new(port);

    server.run();
}
