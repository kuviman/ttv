use super::*;

// TODO
mod font;

mod avatars;
mod boom;
mod hello;
mod jumpscare;
mod raffle_royale;
mod sound_commands;
mod text_commands;
mod util;

use util::*;

#[async_trait(?Send)]
trait Feature: 'static {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized;
    async fn update(&mut self, delta_time: f32);
    async fn handle_event(&mut self, event: geng::Event) {}
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer);
    async fn handle(&mut self, message: &ServerMessage);
}

#[derive(Clone)]
pub struct Connection {
    inner: Arc<Mutex<geng::net::client::Connection<ServerMessage, ClientMessage>>>,
    waiting_for_replies:
        Arc<Mutex<HashMap<String, futures::channel::oneshot::Sender<ServerMessage>>>>,
}

impl Connection {
    fn new(connection: geng::net::client::Connection<ServerMessage, ClientMessage>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(connection)),
            waiting_for_replies: default(),
        }
    }
    fn say(&self, text: &str) {
        self.inner.lock().unwrap().send(ClientMessage::Say {
            text: text.to_owned(),
            reply_to: None,
        });
    }
    fn reply(&self, text: &str, to: &MessageId) {
        self.inner.lock().unwrap().send(ClientMessage::Say {
            text: text.to_owned(),
            reply_to: Some(to.clone()),
        });
    }
    async fn get_key_value<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let request_id: String = thread_rng()
            .sample_iter(rand::distributions::Alphanumeric)
            .map(|c| c as char)
            .take(64)
            .collect();
        let (sender, receiver) = futures::channel::oneshot::channel();
        self.waiting_for_replies
            .lock()
            .unwrap()
            .insert(request_id.clone(), sender);
        self.inner.lock().unwrap().send(ClientMessage::GetKeyValue {
            request_id: request_id.clone(),
            key: format!("{key}.json"),
        });
        let ServerMessage::KeyValue { request_id: response_request_id, value } = receiver.await.unwrap() else {
            unreachable!()
        };
        assert!(request_id == response_request_id);
        value.map(|s| serde_json::from_str(&s).unwrap())
    }
    fn set_key_value<T: Serialize>(&self, key: &str, value: &T) {
        self.inner.lock().unwrap().send(ClientMessage::SetKeyValue {
            key: format!("{key}.json"),
            value: serde_json::to_string(value).unwrap(),
        });
    }
}

struct SyncFeature {
    inner: Option<Box<dyn Feature>>,
    future: Option<Pin<Box<dyn Future<Output = Box<dyn Feature>>>>>,
    messages: std::collections::VecDeque<ServerMessage>,
}

impl SyncFeature {
    fn new(feature: Box<dyn Feature>) -> Self {
        Self {
            inner: Some(feature),
            future: None,
            messages: default(),
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        if let Some(mut inner) = self.inner.take() {
            self.future = Some(
                async move {
                    inner.handle_event(event).await;
                    inner
                }
                .boxed_local(),
            );
        } else {
            // TODO i dont remember what i wanted to do here LUL
        }
    }
    fn update(&mut self, delta_time: f32) {
        if let Some(message) = self.messages.pop_front() {
            if let Some(mut inner) = self.inner.take() {
                self.future = Some(
                    async move {
                        inner.handle(&message).await;
                        inner
                    }
                    .boxed_local(),
                );
            } else {
                self.messages.push_front(message);
            }
        }
        if let Some(mut inner) = self.inner.take() {
            self.future = Some(
                async move {
                    inner.update(delta_time).await;
                    inner
                }
                .boxed_local(),
            );
        }
        if let Some(mut future) = self.future.take() {
            match future.as_mut().poll(&mut std::task::Context::from_waker(
                futures::task::noop_waker_ref(),
            )) {
                std::task::Poll::Ready(state) => {
                    self.inner = Some(state);
                }
                std::task::Poll::Pending => {
                    self.future = Some(future);
                }
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if let Some(inner) = &mut self.inner {
            inner.draw(framebuffer);
        }
    }
    fn handle(&mut self, message: &ServerMessage) {
        self.messages.push_back(message.clone());
    }
}

struct Overlay {
    features: Vec<SyncFeature>,
    connection: Connection,
    receiver: std::sync::mpsc::Receiver<ServerMessage>,
}

impl Overlay {
    pub fn new(geng: &Geng, connection: Connection, features: Vec<Box<dyn Feature>>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || loop {
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).unwrap();
            let line = line.trim();
            let message = if let Some(reward) = line.strip_prefix("!redeem") {
                ServerMessage::RewardRedemption {
                    name: "kuviman".to_owned(),
                    reward: reward.trim().to_owned(),
                }
            } else {
                ServerMessage::ChatMessage {
                    // TODO: rig it so badcop always wins
                    id: MessageId("<fake id>".to_owned()),
                    name: "kuviman".to_owned(),
                    message: line.to_owned(),
                }
            };
            if sender.send(message).is_err() {
                break;
            }
        });
        Self {
            features: features.into_iter().map(SyncFeature::new).collect(),
            connection,
            receiver,
        }
    }
}

impl geng::State for Overlay {
    fn update(&mut self, delta_time: f64) {
        for feature in &mut self.features {
            feature.update(delta_time as f32);
        }
        let mut new_messages = Vec::new();
        for message in self.connection.inner.lock().unwrap().new_messages() {
            new_messages.push(message.unwrap());
        }
        for message in self.receiver.try_iter() {
            new_messages.push(message);
        }
        for message in new_messages {
            if let ServerMessage::KeyValue { request_id, value } = &message {
                self.connection
                    .waiting_for_replies
                    .lock()
                    .unwrap()
                    .remove(request_id)
                    .unwrap()
                    .send(message)
                    .unwrap();
                continue;
            }
            log::info!("{:?}", message);
            for feature in &mut self.features {
                feature.handle(&message);
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.features.iter().any(|feature| feature.inner.is_none()) {
            return;
        }
        ugli::clear(framebuffer, Some(Rgba::new(0.0, 0.0, 0.0, 0.0)), None, None);
        for feature in &mut self.features {
            feature.draw(framebuffer);
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        for feature in &mut self.features {
            feature.handle_event(event.clone());
        }
        // TODO: raffle_royale.handle_event(event)
    }
}

macro_rules! load_features {
    (geng: $geng:expr, connection: $connection:expr, $($feature:ident,)*) => {{
        let geng = $geng;
        let connection = $connection;
        vec![
            $(load_feature::<$feature::State>(&geng, run_dir().join("assets").join(stringify!($feature)), connection.clone()),)*
        ]
    }}
}

pub fn run(geng_args: &geng::CliArgs, addr: &str) {
    let geng = Geng::new_with(geng::ContextOptions {
        title: "TTV".to_owned(),
        transparency: true,
        ..geng::ContextOptions::from_args(geng_args)
    });
    let addr = addr.to_owned();
    geng.clone().run_loading(async move {
        let connection = geng::net::client::connect(&addr).await.unwrap();
        let connection = Connection::new(connection);

        fn load_feature<T: Feature>(
            geng: &Geng,
            path: std::path::PathBuf,
            connection: Connection,
        ) -> Pin<Box<dyn Future<Output = Box<dyn Feature>>>> {
            T::load(geng.clone(), path, connection)
                .map(|feature| Box::new(feature) as Box<dyn Feature>)
                .boxed_local()
        }

        // For pgorley
        Some(Some(Some(Some(Some(Some(()))))))
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();

        let features = future::join_all(load_features![
            geng: &geng,
            connection: connection.clone(),
            avatars,
            raffle_royale,
            boom,
            hello,
            jumpscare,
            sound_commands,
            text_commands,
        ])
        .await;
        Overlay::new(&geng, connection, features)
    });
}
