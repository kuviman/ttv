use super::*;

pub struct Bot {
    config: Config,
    ttv_client: ttv::Client,
    sender: Sender,
    receiver: std::sync::mpsc::Receiver<ClientMessage>,
}

impl Bot {
    pub fn new(
        config: Config,
        ttv_client: ttv::Client,
        sender: Sender,
        receiver: std::sync::mpsc::Receiver<ClientMessage>,
    ) -> Self {
        Self {
            config,
            ttv_client,
            sender,
            receiver,
        }
    }
    pub fn handle_ttv(&mut self, message: ttv::Message) {
        match message {
            ttv::Message::Irc(ttv::IrcMessage::Privmsg(message)) => {
                let mut name = message.sender.name.as_str();
                let mut message_text = message.message_text.as_str();
                if name == self.config.channel_login {
                    if let Some(text) = message_text.strip_prefix("!as") {
                        if let Some((as_name, text)) = text.trim().split_once(' ') {
                            name = as_name.trim();
                            message_text = text.trim();
                        }
                    }
                }
                log::info!("{}", message_text);
                match message_text.trim() {
                    "!gnbadcop" => self.ttv_client.say("Good Night badcop_ rincsDance"),
                    "🦀" => self.ttv_client.say("🦀✅🦀✅🦀✅🦀✅🦀✅🦀✅🦀"),
                    _ => {}
                }
                self.sender.broadcast(ServerMessage::ChatMessage {
                    name: name.to_owned(),
                    message: message_text.to_owned(),
                });
            }
            ttv::Message::RewardRedemption { name, reward } => {
                self.sender
                    .broadcast(ServerMessage::RewardRedemption { name, reward });
            }
            _ => {}
        }
    }
    pub fn run(mut self) {
        loop {
            while let Some(msg) = self.ttv_client.next_message() {
                self.handle_ttv(msg);
            }
            for msg in self.receiver.try_iter() {
                if let ClientMessage::Say { text } = msg {
                    self.ttv_client.say(&text);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
