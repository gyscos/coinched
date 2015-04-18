use hyper;
use std::io::Write;
use super::game_manager::GameManager;

pub struct Server {
    port: u16,
    manager: GameManager,
}

impl hyper::server::Handler for Server {
    fn handle(&self, _: hyper::server::Request, res: hyper::server::Response) {
        let mut res = res.start().unwrap();
        res.write_all(b"Hello World!").unwrap();
        res.end().unwrap();
    }
}

impl Server {
    pub fn new(port: u16) -> Server {
        Server {
            port: port,
            manager: GameManager::new(),
        }
    }

    pub fn run(self) {
        let port = self.port;
        println!("Now listening on port {}", port);
        hyper::Server::http(self).listen(("127.0.0.1", port)).unwrap();
    }
}
