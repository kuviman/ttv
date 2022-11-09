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
    pub fart_color: Rgba<f32>,
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
    pub farticle: ugli::Texture,
    pub boom: ugli::Texture,
    pub boom_sound: geng::Sound,
    #[asset(range = "1..=3", path = "fart/*.wav")]
    fart: Vec<geng::Sound>,
    hello: HelloSounds,
    #[asset(path = "JumpScare1.wav")]
    jumpscare: geng::Sound,
}

struct Hello {
    time: f32,
    name: String,
}

struct Jumpscare {
    time: f32,
}

pub struct Farticle {
    pub size: f32,
    pub pos: Vec2<f32>,
    pub vel: Vec2<f32>,
    pub color: Rgba<f32>,
    pub rot: f32,
    pub w: f32,
    pub t: f32,
}

impl Overlay {
    pub fn update_farticles(&mut self, delta_time: f32) {
        for farticle in &mut self.farticles {
            farticle.t -= delta_time;
            farticle.pos += farticle.vel * delta_time;
            farticle.rot += farticle.w * delta_time;
        }
        self.farticles.retain(|farticle| farticle.t > 0.0);
    }

    pub fn draw_farticles(&self, framebuffer: &mut ugli::Framebuffer) {
        for farticle in &self.farticles {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::unit_colored(
                    &self.assets.farticle,
                    Rgba {
                        a: farticle.color.a * farticle.t,
                        ..farticle.color
                    },
                )
                .transform(Mat3::rotate(farticle.rot))
                .scale_uniform(farticle.size)
                .translate(farticle.pos),
            )
        }
    }
}

struct Boom {
    time: f32,
    pos: Vec2<f32>,
}

struct Overlay {
    framebuffer_size: Vec2<f32>,
    camera: geng::Camera2d,
    assets: Rc<Assets>,
    geng: Geng,
    connection: Connection,
    raffle_royale: RaffleRoyale,
    hello: Option<Hello>,
    font_program: ugli::Program,
    farticles: Vec<Farticle>,
    boom: Option<Boom>,
    jumpscare: Option<Jumpscare>,
}

impl Overlay {
    pub fn new(geng: &Geng, connection: Connection, assets: &Rc<Assets>) -> Self {
        let mut result = Self {
            jumpscare: None,
            framebuffer_size: vec2(1.0, 1.0),
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            assets: assets.clone(),
            geng: geng.clone(),
            connection,
            raffle_royale: RaffleRoyale::new(&geng, &assets.raffle_royale),
            hello: None,
            font_program: geng.shader_lib().compile(font::SHADER_SOURCE).unwrap(),
            farticles: vec![],
            boom: None,
        };
        result
    }
    fn fart(&mut self) {
        let mut effect = self.assets.fart.choose(&mut global_rng()).unwrap().effect();
        effect.set_volume(self.assets.config.volume);
        effect.play();
        let x = global_rng().gen_range(-1.0..1.0) * self.camera.fov / 2.0 * self.framebuffer_size.x
            / self.framebuffer_size.y;
        let y = -self.camera.fov / 2.0;
        for _ in 0..20 {
            self.farticles.push(Farticle {
                size: 0.5,
                pos: vec2(x, y)
                    + vec2(
                        global_rng().gen_range(-1.0..1.0),
                        global_rng().gen_range(-1.0..1.0),
                    ) * 0.5,
                vel: vec2(0.0, 2.0)
                    + vec2(
                        global_rng().gen_range(-1.0..1.0),
                        global_rng().gen_range(-1.0..1.0),
                    ) * 0.5,
                color: self.assets.config.fart_color,
                rot: global_rng().gen_range(0.0..2.0 * f32::PI),
                w: global_rng().gen_range(-1.0..1.0) * 3.0,
                t: 1.0,
            });
        }
    }
}

impl geng::State for Overlay {
    fn update(&mut self, delta_time: f64) {
        self.raffle_royale.update(delta_time);
        let delta_time = delta_time as f32;

        self.update_farticles(delta_time);

        if let Some(hello) = &mut self.hello {
            hello.time += delta_time / 2.0;
            if hello.time > 2.0 {
                self.hello = None;
            }
        }
        if let Some(boom) = &mut self.boom {
            boom.time += delta_time;
            if boom.time > 1.0 {
                self.boom = None;
            }
        }
        if let Some(jumpscare) = &mut self.jumpscare {
            jumpscare.time += delta_time;
            if jumpscare.time > 1.0 {
                self.jumpscare = None;
            }
        }

        for message in self.connection.new_messages().collect::<Vec<_>>() {
            match &message {
                ServerMessage::ChatMessage { name, message } => match message.trim() {
                    "!hellopomo" => {
                        let mut effect = self.assets.hello.pomo.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                    }
                    "!hellopgorley" => {
                        let mut effect = self.assets.hello.pgorley.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                    }
                    "!hellobadcop" => {
                        let mut effect = self.assets.hello.badcop.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                    }
                    "!boom" => {
                        let mut effect = self.assets.boom_sound.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                        self.boom = Some(Boom {
                            time: 0.0,
                            pos: vec2(
                                global_rng().gen_range(-1.0..1.0) * self.framebuffer_size.x
                                    / self.framebuffer_size.y,
                                global_rng().gen_range(-1.0..1.0),
                            ) * self.camera.fov
                                / 2.0
                                * 0.75,
                        });
                    }
                    "!jumpscare" => {
                        let mut effect = self.assets.jumpscare.effect();
                        effect.set_volume(self.assets.config.volume);
                        effect.play();
                        self.jumpscare = Some(Jumpscare { time: 0.0 });
                    }
                    "!fart" => {
                        self.fart();
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
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        ugli::clear(framebuffer, Some(Rgba::TRANSPARENT_WHITE), None, None);
        self.raffle_royale.draw(framebuffer);
        if let Some(hello) = &self.hello {
            let x = self.camera.fov / 2.0 * framebuffer.size().x as f32
                / framebuffer.size().y as f32
                - 2.0;
            let y = -self.camera.fov / 2.0 + 1.0;
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(vec2(x, y - 2.0 * (hello.time - 1.0).abs().sqr()))
                        .extend_uniform(1.0),
                    &self.assets.crab,
                ),
            );
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
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
        if let Some(boom) = &self.boom {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::colored(
                    AABB::point(boom.pos).extend_symmetric(
                        vec2(
                            self.assets.boom.size().x as f32 / self.assets.boom.size().y as f32,
                            1.0,
                        ) * (boom.time + 1.0),
                    ),
                    &self.assets.boom,
                    Rgba::new(1.0, 1.0, 1.0, 1.0 - boom.time),
                ),
            );
        }
        if let Some(_) = &self.jumpscare {
            self.geng.draw_2d(
                framebuffer,
                &self.camera,
                &draw_2d::TexturedQuad::new(
                    AABB::point(Vec2::ZERO).extend_uniform(5.0),
                    &self.assets.yeti,
                ),
            );
        }
        self.draw_farticles(framebuffer);
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
