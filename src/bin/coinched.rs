extern crate coinched;
#[macro_use]
extern crate log;
extern crate env_logger;

fn main() {
    // TODO: read this from arguments
    env_logger::init().unwrap();

    let port = 3000;

    let server = coinched::server::http::Server::new(port);

    server.run();
}
