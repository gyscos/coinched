pub mod server;

pub use self::server::Server;

#[derive(Clone,RustcDecodable,RustcEncodable)]
struct ContractBody {
    target: String,
    suit: u32,
}

#[derive(Clone,RustcDecodable,RustcEncodable)]
struct CardBody {
    card: u32,
}
