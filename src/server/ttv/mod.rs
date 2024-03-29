use super::*;

pub mod auth;

use reqwest::Url;
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
    pub fn new(channel_login: &str, bot_login: &str) -> Self {
        let channel_login = channel_login.to_owned();

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let config = ClientConfig::new_simple(StaticLoginCredentials::new(
            bot_login.to_owned(),
            Some(
                Secrets::init()
                    .unwrap()
                    .ttv_access_token(bot_login)
                    .unwrap(),
            ),
        ));
        log::debug!("Connecting to ttv irc");
        let (mut incoming_messages, client) = tokio_runtime.block_on(async {
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config)
        });
        log::debug!("Connected to ttv irc");

        let (messages_sender, messages_receiver) = tokio::sync::mpsc::unbounded_channel();

        client.join(channel_login.clone()).unwrap();
        let async_thread = {
            let messages_sender = messages_sender.clone();
            async move {
                let join_handle = tokio::spawn(async move {
                    // This loop (and the thread) will only stop when TwitchIRCClient is dropped
                    while let Some(message) = incoming_messages.recv().await {
                        log::trace!("{}", serde_json::to_string(&message).unwrap());
                        if let Err(e) = messages_sender.send(Message::Irc(message)) {
                            log::error!("{:?}", e);
                        }
                    }
                });
                join_handle.await.unwrap();
            }
        };
        let thread = std::thread::spawn(move || {
            log::debug!("Ttv client thread started");
            tokio_runtime.block_on(async_thread);
            log::debug!("Ttv client thread stopped");
        });

        std::thread::spawn({
            let channel_login = channel_login.clone();
            let bot_login = bot_login.to_owned();
            move || {
                log::debug!("PubSub thread started");
                pubsub(
                    &Secrets::init()
                        .unwrap()
                        .ttv_access_token(bot_login)
                        .unwrap(),
                    &channel_login,
                    messages_sender,
                );
                log::debug!("PubSub thread stopped");
            }
        });

        Self {
            channel_login: channel_login.clone(),
            inner: client,
            messages: messages_receiver,
            thread: ThreadJoinHandle {
                inner: Some(thread),
            },
        }
    }

    pub fn wait_next_message(&mut self) -> Option<Message> {
        self.messages.blocking_recv()
    }

    pub fn next_message(&mut self) -> Option<Message> {
        self.messages.try_recv().ok()
    }

    pub fn say(&self, message: &str, reply_to: Option<String>) {
        futures::executor::block_on(self.inner.say_in_response(
            self.channel_login.clone(),
            message.to_owned(),
            reply_to,
        ))
        .unwrap();
    }
}

fn pubsub(access_token: &str, channel_login: &str, sender: UnboundedSender<Message>) {
    let secrets = Secrets::init().unwrap();
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let user_id = tokio_runtime.block_on(async {
        let json = reqwest::Client::new()
            .get("https://api.twitch.tv/helix/users")
            .query(&[("login", channel_login)])
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Client-ID", &secrets.config.ttv.client_id)
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

    block_on(async move {
        let mut ws = websocket_lite::ClientBuilder::new("wss://pubsub-edge.twitch.tv")
            .unwrap()
            .async_connect()
            .await
            .unwrap();
        let request = serde_json::json!({
            "type": "LISTEN",
            "nonce": "kekw",
            "data": {
                "topics": [format!("community-points-channel-v1.{}", user_id)],
                "auth_token": access_token,
            }
        });
        ws.send(websocket_lite::Message::text(
            serde_json::to_string(&request).unwrap(),
        ))
        .await
        .unwrap();
        let mut timer = Timer::new();
        loop {
            if timer.elapsed().as_secs_f64() > 60.0 {
                log::debug!("Sending ping to pubsub");
                ws.send(websocket_lite::Message::text(r#"{"type": "PING"}"#))
                    .await
                    .unwrap();
                timer.tick();
            }
            let message = {
                let message =
                    tokio::time::timeout(std::time::Duration::from_secs(10), ws.next()).await;
                match message {
                    Ok(message) => message,
                    Err(_) => {
                        continue;
                    }
                }
            };
            let message = message.unwrap().unwrap();
            log::debug!("{:?}", message);
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
                if message.get("type").unwrap().as_str().unwrap() == "reward-redeemed" {
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
                    log::info!("{} redeemed {}", name, reward);
                    sender
                        .send(Message::RewardRedemption { name, reward })
                        .unwrap();
                }
            }
        }
    });
}
