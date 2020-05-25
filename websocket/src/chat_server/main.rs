use actix::prelude::*;
use actix_web::{error, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use std::time::{Duration, Instant};
mod server;


const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(5);

struct WsChatSession {
    id: usize,
    hb: Instant,
    room: String,
    name: Option<String>,
    addr: Addr<server::ChatServer>,
}

impl Actor for WsChatSession {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // Register this session to the chat server
        let my_addr = ctx.address();
        self.addr
            .send(server::Connect {
                addr: my_addr.recipient(),
            })
            .into_actor(self) // Converts the future into ActorFuture
            .then(|res, act, ctx| {
                match res {
                    Ok(id) => act.id = id,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!("WS chat session [{}] is stopping", self.id);
        self.addr.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }

}

impl Handler<server::Message> for WsChatSession {
    type Result = ();
    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) -> Self::Result {
        //Handle the message from chat server
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsChatSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        //Handle the messages coming from WS client
        let msg = match msg {
            Err(_) => {
                self.addr.do_send(server::Disconnect { id: self.id });
                ctx.stop();
                return;
            },
            Ok(msg) => msg,
        };

        println!("WEBSOCKET message: {:?}!!!", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                // we check for /sss type of messages
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(2, ' ').collect();
                    match v[0] {
                        "/list" => {
                            // Send ListRooms message to chat server and wait for
                            // response
                            println!("List rooms");
                            self.addr
                                .send(server::ListRooms)
                                .into_actor(self)
                                .then(|res, _, ctx| {
                                    match res {
                                        Ok(rooms) => {
                                            for room in rooms {
                                                ctx.text(room);
                                            }
                                        }
                                        _ => println!("Something is wrong"),
                                    }
                                    fut::ready(())
                                })
                                .wait(ctx)
                            // .wait(ctx) pauses all events in context,
                            // so actor wont receive any new messages until it get list
                            // of rooms back
                        }
                        "/join" => {
                            if v.len() == 2 {
                                self.room = v[1].to_owned();
                                self.addr.do_send(server::Join {
                                    id: self.id,
                                    room: self.room.clone(),
                                });

                                ctx.text("joined");
                            } else {
                                ctx.text("!!! room name is required");
                            }
                        }
                        "/name" => {
                            if v.len() == 2 {
                                self.name = Some(v[1].to_owned());
                            } else {
                                ctx.text("!!! name is required");
                            }
                        }
                        _ => ctx.text(format!("!!! unknown command: {:?}", m)),
                    }
                } else {
                    let msg = if let Some(ref name) = self.name {
                        format!("{}: {}", name, m)
                    } else {
                        m.to_owned()
                    };
                    // send message to chat server
                    self.addr.do_send(server::ClientMessage {
                        id: self.id,
                        msg,
                        room: self.room.clone(),
                    })
                }
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl WsChatSession {
    fn new(srv_addr: Addr<server::ChatServer>) -> Self {
        Self {
            id: 0,
            hb: Instant::now(),
            room: String::from("Main"),
            name: None,
            addr: srv_addr,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        //Start sending heartbeat to WS client
        ctx.run_interval(HEARTBEAT_TIMEOUT, |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                println!("WS session ping timed out, disconnecting...");

                act.addr.do_send(server::Disconnect { id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

async fn chat_route(
    req: HttpRequest,
    srv: web::Data<Addr<server::ChatServer>>,
    stream: web::Payload,
) -> Result<HttpResponse, error::Error> {
    ws::start(WsChatSession::new(srv.get_ref().clone()), &req, stream)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let server = server::ChatServer::default().start();
    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            .service(web::resource("/ws/").route(web::get().to(chat_route)))
    })
    .bind("0.0.0.0:9999")?
    .run()
    .await
}
