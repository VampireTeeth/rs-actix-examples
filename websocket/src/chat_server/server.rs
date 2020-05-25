use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use rand::{ self, rngs::ThreadRng, Rng };

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(result = "usize")]
pub struct Connect {
    pub addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: usize,
    pub msg: String,
    pub room: String,
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct ListRooms;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: usize,
    pub room: String,
}

pub struct ChatServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rooms: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        let mut rooms = HashMap::new();
        rooms.insert(String::from("Main"), HashSet::new());

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
        }
    }
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) -> Self::Result {
        println!("Someone joined lobby");
        self.send_message(&"Main", "Someone joined lobby", 0);
        // Adding a new entry into sessions table
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // automatically join the main room
        self.rooms
            .entry(String::from("Main"))
            .or_insert(HashSet::new())
            .insert(id);

        id
    }

}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) -> Self::Result {
        println!("Someone disconnected");

        let mut rooms: Vec<String> = Vec::new();

        if self.sessions.remove(&msg.id).is_some() {
            for (name, ids) in &mut self.rooms {
                if ids.remove(&msg.id) {
                    rooms.push(String::from(name));
                }
            }
        }

        for room in &rooms {
            self.send_message(room, "Someone disconnected", 0);
        }
    }
}

impl Handler<ListRooms> for ChatServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Self::Context) -> Self::Result {
        let mut rooms: Vec<String> = Vec::new();
        for (room, _) in &self.rooms {
            rooms.push(String::from(room));
        }
        MessageResult(rooms)
    }
}

impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Self::Context) -> Self::Result {
        let Join { id, room } = msg;
        let mut rooms: Vec<String> = Vec::new();

        for (n, ids) in &mut self.rooms {
            if ids.remove(&id) {
                rooms.push(String::from(n));
            }
        }

        for r in &rooms {
            self.send_message(r, "Someone left", 0);
        }

        self.rooms
            .entry(String::from(&room))
            .or_insert(HashSet::new())
            .insert(msg.id);

        self.send_message(&room, "Someone joined", id);
    }
}

impl Handler<ClientMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Self::Context) -> Self::Result {
        self.send_message(&msg.room, &msg.msg, msg.id);
    }
}


impl ChatServer {
    fn send_message(&self, room: &str, msg: &str, skip_id: usize) {
        if let Some(sessions) = self.rooms.get(room) {
            for session_id in sessions {
                if *session_id != skip_id {
                    if let Some(recipient) = self.sessions.get(session_id) {
                        let _ = recipient.do_send(Message(String::from(msg)));
                    }
                }
            }
        }
    }
}
