use super::game_manager::GameManager;

use std::sync::Arc;

use rustc_serialize::json;

use iron;
use iron::prelude::*;

pub struct Server {
    port: u16,
    manager: Arc<GameManager>,
}

struct Router { manager: Arc<GameManager> }

#[derive(RustcEncodable)]
struct HelpAction {
    href: &'static str,
    help: &'static str,
    method: &'static str,
}

#[derive(RustcEncodable)]
struct HelpMessage {
    title: &'static str,
    actions: Vec<HelpAction>,
}

fn help_message() -> String {

    json::encode(&HelpMessage {
        title: "Help Page",
        actions: vec![
            HelpAction {
                href: "/join",
                method: "GET",
                help: "Join a new game.",
            },
            HelpAction {
                href: "/hand/[PLAYER_ID]",
                method: "GET",
                help: "Checks the current hand.",
            },
        ]
    }).unwrap()
}


fn help() -> IronResult<Response> {
    let content_type: iron::mime::Mime = "application/json".parse::<iron::mime::Mime>().unwrap();
    return Ok(Response::with((content_type,
                              iron::status::NotFound,
                              help_message())));
}

impl iron::Handler for Router {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {

        if req.url.path.is_empty() {
            // ?!?
            panic!("Empty request path should never happen.");
        }


        // Weird deref trick to go from &String to &str
        let action: &str = &*req.url.path[0];

        let content_type: iron::mime::Mime = "application/json".parse::<iron::mime::Mime>().unwrap();

        match req.method {
            iron::method::Options => {
                if ["hand", "trick", "contracts", "last_trick", "scores"].contains(&action) {
                    Ok(Response::with((iron::modifiers::Header(
                                           iron::headers::Allow(
                                               vec![
                                                   iron::method::Get,
                                                   iron::method::Options])),
                                       iron::status::Ok)))
                } else if ["pass", "coinche", "bid", "play", "join"].contains(&action) {
                    Ok(Response::with((iron::modifiers::Header(
                                           iron::headers::Allow(
                                               vec![
                                                   iron::method::Post,
                                                   iron::method::Options])),
                                       iron::status::Ok)))
                } else {
                    help()
                }
            },
            iron::method::Get => {
                let response = match action {
                    "hand" => "",
                    "trick" => "",
                    "contracts" => "",
                    "last_trick" => "",
                    "scores" => "",
                    _ => return help(),
                };

                Ok(Response::with((content_type, iron::status::Ok, response)))

            },
            iron::method::Post => {
                // Read the JSON body
                // ...

                let response = match action {
                    "join" => match self.manager.join() {
                        None => "error".to_string(),
                        Some(info) => {
                            #[derive(RustcEncodable)]
                            struct NewPartyInfo {
                                id: u32,
                                pos: usize,
                            }

                            json::encode(&NewPartyInfo {
                                id: info.player_id,
                                pos: info.player_pos.0,
                            }).unwrap()
                        },
                    },
                    _ => return help(),
                };

                Ok(Response::with((content_type, iron::status::Ok, response)))
            },
            _ => help(),
        }
    }
}

impl Server {
    pub fn new(port: u16) -> Server {
        Server {
            port: port,
            manager: Arc::new(GameManager::new()),
        }
    }

    pub fn run(self) {
        let port = self.port;
        println!("Listening on port {}", port);

        let router = Router { manager: self.manager.clone() };

        Iron::new(router).http(("localhost", port)).unwrap();


    }
}
