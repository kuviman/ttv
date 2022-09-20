use std::io::Read;

use crate::Message;
use geng::prelude::futures::executor::block_on;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use twitch_irc::{
    login::StaticLoginCredentials, message::ServerMessage, ClientConfig, SecureTCPTransport,
    TwitchIRCClient,
};

pub struct Client {
    channel_login: String,
    inner: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
    messages: UnboundedReceiver<Message>,
}

impl Client {
    pub fn next_message(&mut self) -> Option<Message> {
        self.messages.try_recv().ok()
    }

    pub fn say(&self, message: &str) {
        block_on(
            self.inner
                .say(self.channel_login.clone(), message.to_owned()),
        )
        .unwrap();
    }
}

pub fn spawn() -> Client {
    let channel_login = "kuviman".to_owned();

    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // default configuration is to join chat as anonymous.
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

    let async_thread = run(
        channel_login.clone(),
        client.clone(),
        incoming_messages,
        messages_sender,
    );
    std::thread::spawn(move || tokio_runtime.block_on(async_thread));

    Client {
        channel_login: channel_login.clone(),
        inner: client,
        messages: messages_receiver,
    }
}

async fn run(
    channel_login: String,
    client: TwitchIRCClient<SecureTCPTransport, StaticLoginCredentials>,
    mut incoming_messages: UnboundedReceiver<ServerMessage>,
    sender: UnboundedSender<Message>,
) {
    // first thing you should do: start consuming incoming messages,
    // otherwise they will back up.
    let join_handle = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            match message {
                // ServerMessage::ClearChat(_) => todo!(),
                // ServerMessage::ClearMsg(_) => todo!(),
                // ServerMessage::GlobalUserState(_) => todo!(),
                // ServerMessage::HostTarget(_) => todo!(),
                // ServerMessage::Join(_) => todo!(),
                // ServerMessage::Notice(_) => todo!(),
                // ServerMessage::Part(_) => todo!(),
                // ServerMessage::Ping(_) => todo!(),
                // ServerMessage::Pong(_) => todo!(),
                ServerMessage::Privmsg(message) => {
                    println!(
                        "{} ({}): {}",
                        message.sender.login, message.sender.name, message.message_text
                    );
                    if message.message_text.trim() == "!fight" {
                        let _ = sender.send(Message::SomeoneWantsToFight {
                            name: message.sender.name,
                        });
                    }
                }
                // ServerMessage::Reconnect(_) => todo!(),
                // ServerMessage::RoomState(_) => todo!(),
                // ServerMessage::UserNotice(_) => todo!(),
                // ServerMessage::UserState(_) => todo!(),
                // ServerMessage::Whisper(_) => todo!(),
                ServerMessage::Notice(notice) => {
                    println!("NOTICE: {}", notice.message_text);
                }
                _ => {
                    // println!("{:?}", message);
                }
            }
        }
    });

    // join a channel
    // This function only returns an error if the passed channel login name is malformed,
    // so in this simple case where the channel name is hardcoded we can ignore the potential
    // error with `unwrap`.
    client.join(channel_login.clone()).unwrap();

    client
        .say(channel_login.clone(), "Online 🤖".to_owned())
        .await
        .unwrap();

    // keep the tokio executor alive.
    // If you return instead of waiting the background task will exit.
    join_handle.await.unwrap();
}
