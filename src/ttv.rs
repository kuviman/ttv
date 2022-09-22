use super::*;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use twitch_irc::{
    login::StaticLoginCredentials,
    message::{PrivmsgMessage, ServerMessage},
    ClientConfig, SecureTCPTransport, TwitchIRCClient,
};

pub type IrcMessage = ServerMessage;

#[derive(Debug)]
pub enum Message {
    Irc(ServerMessage),
    RewardRedemption { name: String, reward: String },
}

// Lets join thread on drop so we shutdown without missing anything
struct ThreadJoinHandle {
    inner: Option<std::thread::JoinHandle<()>>,
}
impl Drop for ThreadJoinHandle {
    fn drop(&mut self) {
        self.inner.take().unwrap().join().unwrap();
    }
}

pub struct Client {
    channel_login: String,
    inner: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
    messages: UnboundedReceiver<Message>,

    // This should be dropped after TwitchIRCClient (so the order of fields is important here),
    // so that the stream of messages is ended and the thread will be stopped
    #[allow(dead_code)] // Used just for drop impl
    thread: ThreadJoinHandle,
}

impl Client {
    pub fn new() -> Self {
        let channel_login = "kuviman".to_owned();

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut token = String::new();
        std::fs::File::open("secret/token")
            .unwrap()
            .read_to_string(&mut token)
            .unwrap();
        token = token.trim().to_owned();
        let config = ClientConfig::new_simple(StaticLoginCredentials::new(
            "kuvibot".to_owned(),
            Some(token),
        ));
        let (mut incoming_messages, client) = tokio_runtime.block_on(async {
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config)
        });

        let (messages_sender, messages_receiver) = tokio::sync::mpsc::unbounded_channel();

        client.join(channel_login.clone()).unwrap();
        let async_thread = {
            let messages_sender = messages_sender.clone();
            async move {
                let join_handle = tokio::spawn(async move {
                    // This loop (and the thread) will only stop when TwitchIRCClient is dropped
                    while let Some(message) = incoming_messages.recv().await {
                        info!("{}", serde_json::to_string(&message).unwrap());
                        if let Err(e) = messages_sender.send(Message::Irc(message)) {
                            error!("{:?}", e);
                        }
                    }
                });
                join_handle.await.unwrap();
            }
        };
        let thread = std::thread::spawn(move || {
            info!("Ttv client thread started");
            tokio_runtime.block_on(async_thread);
            info!("Ttv client thread stopped");
        });

        std::thread::spawn(move || pubsub(messages_sender));

        Self {
            channel_login: channel_login.clone(),
            inner: client,
            messages: messages_receiver,
            thread: ThreadJoinHandle {
                inner: Some(thread),
            },
        }
    }
    pub fn next_message(&mut self) -> Option<Message> {
        self.messages.try_recv().ok()
    }

    pub fn say(&self, message: &str) {
        futures::executor::block_on(
            self.inner
                .say(self.channel_login.clone(), message.to_owned()),
        )
        .unwrap();
    }

    pub fn reply(&self, message: &str, to: &PrivmsgMessage) {
        futures::executor::block_on(self.inner.reply_to_privmsg(message.to_owned(), to)).unwrap();
    }
}

#[derive(Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
}

fn read_file(path: &str) -> String {
    let mut result = String::new();
    std::fs::File::open(path)
        .unwrap()
        .read_to_string(&mut result)
        .unwrap();
    result
}

pub fn refresh_token() {
    let token_data: TokenData =
        serde_json::from_reader(std::fs::File::open("secret/token.json").unwrap()).unwrap();
    std::fs::copy("secret/token.json", "secret/old_token.json").unwrap();
    let mut form = HashMap::new();
    form.insert("client_id", read_file("secret/client_id"));
    form.insert("client_secret", read_file("secret/client_secret"));
    form.insert("grant_type", "refresh_token".to_owned());
    form.insert("refresh_token", token_data.refresh_token);

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let new_token_data = tokio_runtime.block_on(async {
        reqwest::Client::new()
            .post("https://id.twitch.tv/oauth2/token")
            .form(&form)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    });
    std::fs::File::create("secret/token.json")
        .unwrap()
        .write_all(new_token_data.as_bytes())
        .unwrap();
}

fn pubsub(sender: UnboundedSender<Message>) {
    let token_data: ttv::TokenData =
        serde_json::from_reader(std::fs::File::open("secret/token.json").unwrap()).unwrap();
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let user_id = tokio_runtime.block_on(async {
        info!("Sending request");
        let json = reqwest::Client::new()
            .get("https://api.twitch.tv/helix/users")
            .query(&[("login", "kuviman")])
            .header(
                "Authorization",
                format!("Bearer {}", token_data.access_token),
            )
            .header("Client-ID", read_file("secret/client_id"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        serde_json::from_str::<serde_json::Value>(&json)
            .unwrap()
            .as_object()
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .as_object()
            .unwrap()
            .get("id")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    });

    let mut ws = websocket_lite::ClientBuilder::new("wss://pubsub-edge.twitch.tv")
        .unwrap()
        .connect()
        .unwrap();
    let request = serde_json::json!({
        "type": "LISTEN",
        "nonce": "kekw",
        "data": {
            "topics": [format!("channel-points-channel-v1.{}", user_id)],
            "auth_token": token_data.access_token,
        }
    });
    ws.send(websocket_lite::Message::text(
        serde_json::to_string(&request).unwrap(),
    ))
    .unwrap();
    while let Ok(Some(message)) = ws.receive() {
        let message = serde_json::from_str::<serde_json::Value>(message.as_text().unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        if message.get("type").unwrap() == "MESSAGE" {
            let message = message
                .get("data")
                .unwrap()
                .as_object()
                .unwrap()
                .get("message")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let message = serde_json::from_str::<serde_json::Value>(&message)
                .unwrap()
                .as_object()
                .unwrap()
                .clone();
            let data = message
                .get("data")
                .unwrap()
                .as_object()
                .unwrap()
                .get("redemption")
                .unwrap()
                .as_object()
                .unwrap();
            let name = data
                .get("user")
                .unwrap()
                .as_object()
                .unwrap()
                .get("display_name")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            let reward = data
                .get("reward")
                .unwrap()
                .as_object()
                .unwrap()
                .get("title")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned();
            info!("{} redeemed {}", name, reward);
            sender
                .send(Message::RewardRedemption { name, reward })
                .unwrap();
        }
    }
}
