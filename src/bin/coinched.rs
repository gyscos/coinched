extern crate coinched;

fn main() {
    let server = coinched::http::Server::new(3000);

    server.run();
}
