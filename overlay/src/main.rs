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

trait Feature: 'static {
    fn load(geng: Geng, path: std::path::PathBuf) -> Pin<Box<dyn Future<Output = Self>>>
    where
        Self: Sized;
    fn update(&mut self, delta_time: f32);
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer);
    fn handle(&mut self, message: &ServerMessage);
}

struct Overlay {
    features: Vec<Box<dyn Feature>>,
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
            features,
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
