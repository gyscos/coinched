use super::game_manager::GameManager;
use {ContractBody, CardBody};

use std::sync::Arc;
use std::str::FromStr;

use rustc_serialize::json;
use iron::prelude::*;
use iron;
use bodyparser;

struct Router {
    manager: Arc<GameManager>,
}

#[derive(RustcEncodable)]
struct HelpAction {
    href: &'static str,
    method: &'static str,
    help: &'static str,
}

#[derive(RustcEncodable)]
struct HelpMessage {
    title: &'static str,
    actions: Vec<HelpAction>,
}

pub struct Server {
    port: u16,
    manager: Arc<GameManager>,
}

fn help_message() -> String {

    json::encode(&HelpMessage {
        title: "Help Page",
        actions: vec![
            HelpAction {
                href: "/join",
                method: "POST",
                help: "Join a new game.",
            },
            HelpAction {
                href: "/leave/[PLAYER_ID]",
                method: "POST",
                help: "Leave the current game.",
            },
            HelpAction {
                href: "/pass/[PLAYER_ID]",
                method: "POST",
                help: "Pass during auction.",
            },
            HelpAction {
                href: "/coinche/[PLAYER_ID]",
                method: "POST",
                help: "Coinche the opponent's bid during auction.",
            },
            HelpAction {
                href: "/bid/[PLAYER_ID]",
                method: "POST",
                help: "Bid a contract during auction.",
            },
            HelpAction {
                href: "/play/[PLAYER_ID]",
                method: "POST",
                help: "Play a card.",
            },
            HelpAction {
                href: "/hand/[PLAYER_ID]",
                method: "GET",
                help: "Checks the current hand.",
            },
            HelpAction {
                href: "/trick/[PLAYER_ID]",
                method: "GET",
                help: "Checks the current trick.",
            },
            HelpAction {
                href: "/last_trick/[PLAYER_ID]",
                method: "GET",
                help: "Checks the last complete trick.",
            },
            HelpAction {
                href: "/scores/[PLAYER_ID]",
                method: "GET",
                help: "Get the current scores.",
            },
            HelpAction {
                href: "/pos/[PLAYER_ID]",
                method: "GET",
                help: "Get the player's position on the table.",
            },
            HelpAction {
                href: "/wait/[PLAYER_ID]/[EVENT_ID]",
                method: "GET",
                help: "Wait until the next event, or return it if it already happened.",
            },
        ],
    })
        .unwrap()
}


fn help_resp() -> IronResult<Response> {
    let content_type: iron::mime::Mime = "application/json".parse::<iron::mime::Mime>().unwrap();
    return Ok(Response::with((content_type, iron::status::NotFound, help_message())));
}

fn err_resp(msg: &str) -> IronResult<Response> {
    let content_type: iron::mime::Mime = "application/json".parse::<iron::mime::Mime>().unwrap();

    #[derive(RustcEncodable)]
    struct Error<'a> {
        error: &'a str,
    }

    return Ok(Response::with((content_type,
                              iron::status::Ok,
                              json::encode(&Error { error: msg }).unwrap())));
}

macro_rules! parse_id {
    ( $name:expr, $value:expr ) => {
        {
            match u32::from_str($value) {
                Ok(id) => id,
                Err(e) => return err_resp(&format!("invalid {} ID: `{}` ({})", $name, $value, e)),
            }
        }
    };
}

macro_rules! check_len {
    ( $path:expr, 1 ) => {
        {
            if $path.len() != 1 {
                return err_resp(&format!("incorrect parameters (Usage: /{})", $path[0]));
            }
        }
    };
    ( $path:expr, 2 ) => {
        {
            if $path.len() != 2 {
                return err_resp(&format!("incorrect parameters (Usage: /{}/[PID])", $path[0]));
            }
        }
    };
    ( $path:expr, 3 ) => {
        {
            if $path.len() != 3 {
                return err_resp(&format!(
                        "incorrect parameters (Usage: /{}/[PID]/[EID])",
                        $path[0]));
            }
        }
    };
}

macro_rules! my_try {
    ( $x:expr ) => {

        {
            match $x {
                Err(err) => return err_resp(&format!("{}", err)),
                Ok(thing) => thing,
            }
        }
    };
}

macro_rules! try_manager {
    ( $call:expr ) => {
        json::encode(&my_try!($call)).unwrap()
    };
}

macro_rules! read_body {
    ( $x:expr, $name:expr ) => {
        {
            match $x {
                Ok(Some(thing)) => thing,
                Ok(None) => return err_resp(&format!("body expected: {}", $name)),
                Err(err) => return err_resp(&format!("Error parsing {}: {:?}", $name, err)),
            }
        }
    };
}

impl iron::Handler for Router {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        trace!("Router::handle()");

        if req.url.path.is_empty() {
            // ?!?
            panic!("Empty request path should never happen.");
        }


        // Weird deref trick to go from &String to &str

        trace!("Building ContentType");
        let content_type: iron::mime::Mime = "application/json"
                                                 .parse::<iron::mime::Mime>()
                                                 .unwrap();

        trace!("Request: {:?}", req);
        match req.method {
            iron::method::Options => {
                let action = &*req.url.path[0];
                if ["hand", "trick", "last_trick", "scores", "pos"].contains(&action) {
                    Ok(Response::with((iron::modifiers::Header(iron::headers::Allow(vec![
                                                   iron::method::Get,
                                                   iron::method::Options])),
                                       iron::status::Ok)))
                } else if ["pass", "coinche", "bid", "play", "join", "leave"].contains(&action) {
                    Ok(Response::with((iron::modifiers::Header(iron::headers::Allow(vec![
                                                   iron::method::Post,
                                                   iron::method::Options])),
                                       iron::status::Ok)))
                } else {
                    help_resp()
                }
            }
            iron::method::Get => {
                let response = match &*req.url.path[0] {
                    "wait" => {
                        check_len!(req.url.path, 3);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        let event_id = parse_id!("event", &*req.url.path[2]) as usize;
                        // Result is an Event
                        try_manager!(self.manager.wait(player_id, event_id))
                    }
                    "hand" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is a cards::Hand = u32
                        try_manager!(self.manager.see_hand(player_id))
                    }
                    "trick" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is a trick::Trick
                        try_manager!(self.manager.see_trick(player_id))
                    }
                    "last_trick" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is a trick::Trick
                        try_manager!(self.manager.see_last_trick(player_id))
                    }
                    "scores" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is a [i32; 2]
                        try_manager!(self.manager.see_scores(player_id))
                    }
                    "pos" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is a pos::PlayerPos = usize
                        try_manager!(self.manager.see_pos(player_id))
                    }
                    _ => {
                        trace!("Requesting invalid path: GET {:?}", &req.url.path);
                        return help_resp();
                    }
                };

                Ok(Response::with((content_type, iron::status::Ok, response)))

            }
            iron::method::Post => {
                // Read the JSON body
                // ...

                let response = match &*req.url.path[0] {
                    "join" => {
                        check_len!(req.url.path, 1);
                        // Result is a NewPartyInfo
                        try_manager!(self.manager.join())
                    }
                    "leave" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        my_try!(self.manager.leave(player_id));
                        // Result is a string - but who cares?
                        r#""ok""#.to_string()
                    }
                    "pass" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is an event
                        try_manager!(self.manager.pass(player_id))
                    }
                    "coinche" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Result is an event
                        try_manager!(self.manager.coinche(player_id))
                    }
                    "bid" => {
                        trace!("Request: POST /bid");
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        trace!("bid from {}", player_id);
                        // Parse the body

                        let contract = read_body!(req.get::<bodyparser::Struct<ContractBody>>(),
                                                  "contract");
                        trace!("Bidding {:?}", contract);
                        // Result is an event
                        try_manager!(self.manager.bid(player_id, contract))
                    }
                    "play" => {
                        check_len!(req.url.path, 2);
                        let player_id = parse_id!("player", &*req.url.path[1]);
                        // Parse the body
                        let card = read_body!(req.get::<bodyparser::Struct<CardBody>>(), "card");

                        // Result is an event
                        try_manager!(self.manager.play_card(player_id, card))
                    }
                    _ => {
                        trace!("Requesting invalid path: POST {:?}", &req.url.path);
                        return help_resp();
                    }
                };

                Ok(Response::with((content_type, iron::status::Ok, response)))
            }
            _ => {
                trace!("Requesting invalid path: {:?} {:?}",
                       req.method,
                       &req.url.path);
                return help_resp();
            }
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
