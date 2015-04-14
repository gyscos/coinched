use hyper;
use super::game_manager::GameManager;

pub struct HttpServer {
    manager: GameManager,
}

impl HttpServer {
    pub fn new() -> HttpServer {
        HttpServer {
            manager: GameManager::new(),
        }
    }

    pub fn run(&mut self) {
    }
}
