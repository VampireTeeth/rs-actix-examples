use actix::*;
use actix::io::{SinkWrite, WriteHandler};
use actix_codec::{Framed};
use awc::{Client, BoxedSocket, ws::{Message, Frame, Codec}, error::WsProtocolError};
use futures::stream::{StreamExt, SplitSink};
use bytes::Bytes;
use std::time::Duration;
use std::{thread, io};

fn main() {
    ::std::env::set_var("RUST_LOG", "actix-server=info,actix-web=info");
    env_logger::init();

    let sys = System::new("websocket-client");
    Arbiter::spawn(async {
        let (response, framed) = Client::new()
            .ws("http://127.0.0.1:9999/ws/")
        // pub async fn connect(mut self,) -> Result<(ClientResponse, Framed<BoxedSocket, Codec>), WsClientError> {
            .connect()
            .await
            .map_err(|e| {
                print!("Error: {}", e);
            })
            .unwrap();
        println!("{:?}", response);
        // Create ChatClient actor
        let (sink, stream) = framed.split();
        let addr = ChatClient::create(|ctx| {
            ChatClient::add_stream(stream, ctx);
            ChatClient(SinkWrite::new(sink, ctx))
        });
        // Start the console loop
        thread::spawn(move || {
            loop {
                let mut cmd = String::new();
                if io::stdin().read_line(&mut cmd).is_err() {
                    println!("Error when collecting user input");
                    return;
                }
                addr.do_send(ClientCommand(cmd));
            }
        });

    });
    sys.run().unwrap();
}

struct ChatClient(SinkWrite<Message, SplitSink<Framed<BoxedSocket, Codec>, Message>>);

#[derive(Message)]
#[rtype(result = "()")]
struct ClientCommand(String);

impl ChatClient {
    fn hb(&mut self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_later(Duration::new(1, 0), |act, ctx| {
            act.0.write(Message::Ping(Bytes::from_static(b""))).unwrap();
            act.hb(ctx);
        });
    }
}

impl Actor for ChatClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Chat client started, start heartbeats...");
        self.hb(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("Chat client stopped");
        System::current().stop();
    }
}

impl Handler<ClientCommand> for ChatClient {
    type Result = ();

    fn handle(&mut self, msg: ClientCommand, _ctx: &mut Self::Context) -> Self::Result {
        self.0.write(Message::Text(msg.0)).unwrap();
    }
}

impl StreamHandler<Result<Frame, WsProtocolError>> for ChatClient {
    fn handle(&mut self, msg: Result<Frame, WsProtocolError>, _: &mut Self::Context) {
        if let Ok(Frame::Text(text)) = msg {
            println!("Server: {:?}", text);
        }
    }

    fn started(&mut self, _: &mut Self::Context) {
        println!("WS connected");
    }

    fn finished(&mut self, ctx: &mut Self::Context) {
        println!("WS disconnected");
        ctx.stop();
    }
}

impl WriteHandler<WsProtocolError> for ChatClient {}
