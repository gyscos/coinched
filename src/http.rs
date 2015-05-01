use super::game_manager::GameManager;

pub struct Server {
    port: u16,
    manager: GameManager,
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
    }
}
