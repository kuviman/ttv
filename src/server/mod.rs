use super::*;

mod bot;
mod secret;
mod ttv;
mod util;

use secret::Secrets;
use util::*;

#[derive(Deserialize)]
pub struct Config {
    pub channel_login: String,
    pub bot_login: String,
}

#[derive(Clone)]
pub struct Sender {
    senders: Arc<Mutex<Vec<Arc<Mutex<Box<dyn geng::net::Sender<ServerMessage>>>>>>>,
}

impl Sender {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(Mutex::new(vec![])),
        }
    }
    pub fn broadcast(&self, message: ServerMessage) {
        for sender in &mut *self.senders.lock().unwrap() {
            sender.lock().unwrap().send(message.clone());
        }
    }

    pub fn register(&self, sender: Arc<Mutex<Box<dyn geng::net::Sender<ServerMessage>>>>) {
        self.senders.lock().unwrap().push(sender);
    }
}

pub fn run(addr: &str, serve_path: Option<&std::path::Path>) {
    let config: Config =
        serde_json::from_reader(std::fs::File::open("config.json").unwrap()).unwrap();
    let mut ttv = ttv::Client::new(&config.channel_login, &config.bot_login);

    struct WsClient {
        sender: Arc<Mutex<Box<dyn geng::net::Sender<ServerMessage>>>>,
        bot_sender: std::sync::mpsc::Sender<ClientMessage>,
    }

    impl Drop for WsClient {
        fn drop(&mut self) {
            // todo!()
        }
    }

    impl geng::net::Receiver<ClientMessage> for WsClient {
        fn handle(&mut self, message: ClientMessage) {
            fn key_file_path(key: &str) -> std::path::PathBuf {
                std::path::Path::new("storage").join(key)
            }
            match message {
                ClientMessage::GetKeyValue { request_id, key } => {
                    let value = match std::fs::File::open(key_file_path(&key)) {
                        Ok(mut file) => {
                            let mut result = String::new();
                            file.read_to_string(&mut result);
                            Some(result)
                        }
                        Err(e) => None,
                    };
                    self.sender
                        .lock()
                        .unwrap()
                        .send(ServerMessage::KeyValue { request_id, value });
                }
                ClientMessage::SetKeyValue { key, value } => {
                    let path = key_file_path(&key);
                    std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                    std::fs::File::create(path)
                        .unwrap()
                        .write_all(value.as_bytes())
                        .unwrap();
                }
                ClientMessage::Say { text } => {
                    self.bot_sender.send(ClientMessage::Say { text });
                }
            }
        }
    }

    struct WsApp {
        sender: Sender,
        bot_sender: std::sync::mpsc::Sender<ClientMessage>,
    }

    impl WsApp {
        pub fn new(sender: Sender, bot_sender: std::sync::mpsc::Sender<ClientMessage>) -> Self {
            Self { sender, bot_sender }
        }
    }

    impl geng::net::server::App for WsApp {
        type Client = WsClient;
        type ServerMessage = ServerMessage;
        type ClientMessage = ClientMessage;
        fn connect(&mut self, sender: Box<dyn geng::net::Sender<ServerMessage>>) -> WsClient {
            let sender = Arc::new(Mutex::new(sender));
            self.sender.register(sender.clone());
            WsClient {
                sender,
                bot_sender: self.bot_sender.clone(),
            }
        }
    }

    let sender = Sender::new();
    let (bot_sender, bot_receiver) = std::sync::mpsc::channel();

    std::thread::spawn({
        let sender = sender.clone();
        let addr = addr.to_owned();
        move || {
            geng::net::Server::new(WsApp::new(sender, bot_sender), &addr).run();
        }
    });

    // TODO: do I need this?
    #[cfg(feature = "serve")]
    std::thread::spawn({
        let serve_path = serve_path.map(|path| path.to_owned());
        move || {
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
                    let addr = ([0, 0, 0, 0], SERVER_PORT).into();
                    // let addr = ([127, 0, 0, 1], SERVER_PORT).into();
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

            if let Some(path) = serve_path {
                serve(path, false);
            }
        }
    });

    let bot = bot::Bot::new(config, ttv, sender, bot_receiver);
    bot.run();
}
