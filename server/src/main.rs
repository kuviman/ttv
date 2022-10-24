use geng::prelude::batbox::{self, prelude::*};
use serde::{Deserialize, Serialize};

mod bot;
mod secret;
mod ttv;
mod util;

use interop::*;
use secret::Secrets;
use util::*;

#[derive(Deserialize)]
pub struct Config {
    pub channel_login: String,
    pub bot_login: String,
}

#[derive(Clone)]
pub struct Sender {
    senders: Arc<Mutex<Vec<Box<dyn geng::net::Sender<ServerMessage>>>>>,
}

impl Sender {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(vec![])),
        }
    }
    pub fn broadcast(&self, message: ServerMessage) {
        for sender in &mut *self.senders.lock().unwrap() {
            sender.send(message.clone());
        }
    }

    pub fn register(&self, sender: Box<dyn geng::net::Sender<ServerMessage>>) {
        self.senders.lock().unwrap().push(sender);
    }
}

fn main() {
    {
        let mut builder = logger::builder();
        builder.parse_filters("geng=info");
        builder.parse_filters("sqlx=off");
        builder.parse_filters("reqwest=off");
        logger::init_with(builder).unwrap();
    }

    let config: Config =
        serde_json::from_reader(std::fs::File::open("config.json").unwrap()).unwrap();
    let mut ttv = ttv::Client::new(&config.channel_login, &config.bot_login);

    struct WsClient {}

    impl Drop for WsClient {
        fn drop(&mut self) {
            // todo!()
        }
    }

    impl geng::net::Receiver<ClientMessage> for WsClient {
        fn handle(&mut self, message: ClientMessage) {
            // todo!()
        }
    }

    struct WsApp {
        sender: Sender,
    }

    impl WsApp {
        pub fn new(sender: Sender) -> Self {
            Self { sender }
        }
    }

    impl geng::net::server::App for WsApp {
        type Client = WsClient;
        type ServerMessage = ServerMessage;
        type ClientMessage = ClientMessage;
        fn connect(&mut self, sender: Box<dyn geng::net::Sender<ServerMessage>>) -> WsClient {
            self.sender.register(sender);
            WsClient {}
        }
    }

    let sender = Sender::new();

    std::thread::spawn({
        let sender = sender.clone();
        || {
            geng::net::Server::new(WsApp::new(sender), "127.0.0.1:1001").run();
        }
    });

    std::thread::spawn(|| {
        fn serve<P>(dir: P, open: bool)
        where
            std::path::PathBuf: From<P>,
        {
            use hyper::service::{make_service_fn, service_fn};
            use hyper::{Body, Request, Response};
            use hyper_staticfile::Static;
            use std::io::Error as IoError;

            async fn handle_request<B>(
                req: Request<B>,
                static_: Static,
            ) -> Result<Response<Body>, IoError> {
                static_.clone().serve(req).await
            }

            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let static_ = Static::new(dir);

                let make_service = make_service_fn(|_| {
                    let static_ = static_.clone();
                    future::ok::<_, hyper::Error>(service_fn(move |req| {
                        handle_request(req, static_.clone())
                    }))
                });

                const SERVER_PORT: u16 = 8000;
                // let addr = ([0, 0, 0, 0], SERVER_PORT).into();
                let addr = ([127, 0, 0, 1], SERVER_PORT).into();
                let server = hyper::server::Server::bind(&addr).serve(make_service);
                let addr = format!("http://{}/", addr);
                eprintln!("Server running on {}", addr);
                if open {
                    open::that(format!("http://localhost:{}", SERVER_PORT))
                        .expect("Failed to open browser");
                }
                server.await.expect("Server failed");
            });
        }

        serve("target/geng", false);
    });

    let bot = bot::Bot::new(config, ttv, sender);
    bot.run();
}
