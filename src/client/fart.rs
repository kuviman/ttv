use super::*;

#[derive(Deserialize, geng::Assets)]
#[asset(json)]
struct Config {
    fart_color: Rgba<f32>,
}

#[derive(geng::Assets)]
struct Assets {
    config: Config,
    #[asset(range = "1..=3", path = "*.wav")]
    fart: Vec<geng::Sound>,
    farticle: ugli::Texture,
}

struct Farticle {
    size: f32,
    pos: Vec2<f32>,
    vel: Vec2<f32>,
    color: Rgba<f32>,
    rot: f32,
    w: f32,
    t: f32,
}

pub struct State {
    geng: Geng,
    framebuffer_size: Vec2<f32>,
    assets: Assets,
    camera: geng::Camera2d,
    farticles: Vec<Farticle>,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, assets_path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            assets: geng::LoadAsset::load(&geng, &assets_path).await.unwrap(),
            framebuffer_size: vec2(1.0, 1.0),
            geng,
            camera: geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            farticles: vec![],
        }
    }

    async fn update(&mut self, delta_time: f32) {
        for farticle in &mut self.farticles {
            farticle.t -= delta_time;
            farticle.pos += farticle.vel * delta_time;
            farticle.rot += farticle.w * delta_time;
        }
        self.farticles.retain(|farticle| farticle.t > 0.0);
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
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

    async fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        if message.trim() != "!fart" {
            return;
        }
        self.assets.fart.choose(&mut global_rng()).unwrap().play();
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
