use geng::prelude::*;

use interop::*;

// TODO
// mod font;
mod raffle_royale;

use raffle_royale::Assets;
use raffle_royale::RaffleRoyale;

type Connection = geng::net::client::Connection<ServerMessage, ClientMessage>;

struct Overlay {
    assets: Rc<Assets>,
    geng: Geng,
    connection: Connection,
    background_color: Rgba<f32>,
    raffle_royale: RaffleRoyale,
}

impl Overlay {
    pub fn new(geng: &Geng, connection: Connection, assets: &Rc<Assets>) -> Self {
        Self {
            assets: assets.clone(),
            geng: geng.clone(),
            connection,
            background_color: "#123456".try_into().unwrap(),
            raffle_royale: RaffleRoyale::new(&geng, assets),
        }
    }
}

impl geng::State for Overlay {
    fn update(&mut self, delta_time: f64) {
        self.raffle_royale.update(delta_time);
        for message in self.connection.new_messages() {
            self.raffle_royale.handle_message(message);
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(self.background_color), None, None);
        self.raffle_royale.draw(framebuffer);
    }
    fn handle_event(&mut self, event: geng::Event) {
        self.raffle_royale.handle_event(event);
    }
}

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    connect: Option<String>,
}

fn main() {
    let geng = Geng::new("TTV");
    let opt: Opt = program_args::parse();
    let connection =
        geng::net::client::connect(opt.connect.as_deref().unwrap_or("ws://127.0.0.1:1001"));
    let assets = <Assets as geng::LoadAsset>::load(&geng, &static_path());
    geng::run(
        &geng,
        geng::LoadingScreen::new(
            &geng,
            geng::EmptyLoadingScreen,
            future::join(connection, assets),
            {
                let geng = geng.clone();
                move |(connection, assets)| {
                    let mut assets = assets.unwrap();
                    assets.process();
                    Overlay::new(&geng, connection, &Rc::new(assets))
                }
            },
        ),
    );
}
