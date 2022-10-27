use geng::prelude::*;

use interop::*;

// TODO
mod font;
mod raffle_royale;

use raffle_royale::RaffleRoyale;

type Connection = geng::net::client::Connection<ServerMessage, ClientMessage>;

#[derive(Deserialize, geng::Assets)]
#[asset(json)]
pub struct Config {
    pub volume: f64,
}

#[derive(geng::Assets)]
pub struct HelloSounds {
    pomo: geng::Sound,
    badcop: geng::Sound,
    #[asset(range = "1..=3", path = "*.wav")]
    regular: Vec<geng::Sound>,
}

#[derive(geng::Assets)]
pub struct Assets {
    raffle_royale: Rc<raffle_royale::Assets>,
    pub config: Config,
    pub crab: ugli::Texture,
    pub yeti: ugli::Texture,
    hello: HelloSounds,
}

struct Hello {
    time: f32,
    name: String,
}

struct Overlay {
    assets: Rc<Assets>,
    geng: Geng,
    connection: Connection,
    raffle_royale: RaffleRoyale,
    hello: Option<Hello>,
    font_program: ugli::Program,
}

impl Overlay {
    pub fn new(geng: &Geng, connection: Connection, assets: &Rc<Assets>) -> Self {
        Self {
            assets: assets.clone(),
            geng: geng.clone(),
            connection,
            raffle_royale: RaffleRoyale::new(&geng, &assets.raffle_royale),
            hello: None,
            font_program: geng.shader_lib().compile(font::SHADER_SOURCE).unwrap(),
        }
    }
}

impl geng::State for Overlay {
    fn update(&mut self, delta_time: f64) {
        self.raffle_royale.update(delta_time);
        let delta_time = delta_time as f32;

        if let Some(hello) = &mut self.hello {
            hello.time += delta_time / 2.0;
            if hello.time > 2.0 {
                self.hello = None;
            }
        }

        for message in self.connection.new_messages() {
            match &message {
                ServerMessage::ChatMessage { name, message } => match message.trim() {
                    "!hellopomo" => {
                        let mut effect = self.assets.hello.pomo.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                    }
                    "!hellobadcop" => {
                        let mut effect = self.assets.hello.badcop.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                    }
                    _ => {}
                },
                ServerMessage::RewardRedemption { name, reward } => match reward.as_str() {
                    "Hello" => {
                        let mut effect = self
                            .assets
                            .hello
                            .regular
                            .choose(&mut global_rng())
                            .unwrap()
                            .effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                        self.hello = Some(Hello {
                            time: 0.0,
                            name: name.clone(),
                        });
                    }
                    _ => {
                        warn!("Unhandled reward")
                    }
                },
            }
            self.raffle_royale.handle_message(message);
        }
    }
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Rgba::TRANSPARENT_BLACK), None, None);
        self.raffle_royale.draw(framebuffer);
        let camera = geng::Camera2d {
            center: Vec2::ZERO,
            rotation: 0.0,
            fov: 10.0,
        };
        if let Some(hello) = &self.hello {
            let x =
                camera.fov / 2.0 * framebuffer.size().x as f32 / framebuffer.size().y as f32 - 2.0;
            let y = -camera.fov / 2.0 + 1.0;
            self.geng.draw_2d(
                framebuffer,
                &camera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(vec2(x, y - 2.0 * (hello.time - 1.0).abs().sqr()))
                        .extend_uniform(1.0),
                    &self.assets.crab,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &camera,
                &font::Text::unit(
                    &self.geng,
                    &self.font_program,
                    &**self.geng.default_font(),
                    format!("Hello, {}", hello.name),
                    Rgba::WHITE,
                    Rgba::BLACK,
                )
                .scale_uniform(0.15)
                .translate(vec2(x, y - 0.5)),
            );
        }
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
    let geng = Geng::new_with(geng::ContextOptions {
        title: "TTV".to_owned(),
        transparency: true,
        ..default()
    });
    let opt: Opt = program_args::parse();
    let connection =
        geng::net::client::connect(opt.connect.as_deref().unwrap_or("ws://127.0.0.1:8001"));
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
                    // assets.process(); // TODO
                    Overlay::new(&geng, connection, &Rc::new(assets))
                }
            },
        ),
    );
}
