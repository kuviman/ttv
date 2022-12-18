#![deny(unused_must_use)]
use geng::prelude::*;

use interop::*;

// TODO
mod font;

mod boom;
mod fart;
mod hello;
mod jumpscare;
mod raffle_royale;
mod sound_commands;

type Connection = geng::net::client::Connection<ServerMessage, ClientMessage>;

#[async_trait(?Send)]
trait Feature: 'static {
    async fn load(geng: Geng, path: std::path::PathBuf) -> Self
    where
        Self: Sized;
    async fn update(&mut self, delta_time: f32);
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer);
    async fn handle(&mut self, message: &ServerMessage);
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
    connection: Option<Connection>,
    receiver: std::sync::mpsc::Receiver<ServerMessage>,
}

impl Overlay {
    pub fn new(
        geng: &Geng,
        connection: Option<Connection>,
        features: Vec<Box<dyn Feature>>,
    ) -> Self {
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
        if let Some(connection) = &mut self.connection {
            for message in connection.new_messages() {
                new_messages.push(message);
            }
        }
        for message in self.receiver.try_iter() {
            new_messages.push(message);
        }
        for message in new_messages {
            info!("{:?}", message);
            for feature in &mut self.features {
                feature.handle(&message);
            }
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::TRANSPARENT_WHITE), None, None);
        for feature in &mut self.features {
            feature.draw(framebuffer);
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        // TODO: raffle_royale.handle_event(event)
    }
}

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    connect: Option<String>,
}

fn main() {
    logger::init().unwrap();
    geng::setup_panic_handler();
    let geng = Geng::new_with(geng::ContextOptions {
        title: "TTV".to_owned(),
        transparency: true,
        ..default()
    });
    let opt: Opt = program_args::parse();
    let connection = future::OptionFuture::from(
        opt.connect
            .as_deref()
            .map(|path| geng::net::client::connect(path)),
    );
    fn load_feature<T: Feature>(
        geng: &Geng,
        path: std::path::PathBuf,
    ) -> Pin<Box<dyn Future<Output = Box<dyn Feature>>>> {
        T::load(geng.clone(), path)
            .map(|feature| Box::new(feature) as Box<dyn Feature>)
            .boxed_local()
    }
    macro_rules! load_features {
        ($($feature:ident,)*) => {
            vec![
                $(load_feature::<$feature::State>(&geng, static_path().join(stringify!($feature))),)*
            ]
        }
    }
    let features = future::join_all(load_features![
        // TODO raffle_royale,
        boom,
        fart,
        hello,
        jumpscare,
        sound_commands,
    ]);
    geng::run(
        &geng,
        geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            future::join(connection, features),
            {
                let geng = geng.clone();
                move |(connection, features)| Overlay::new(&geng, connection, features)
            },
        ),
    );
}
