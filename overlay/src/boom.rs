use super::*;

#[derive(geng::Assets)]
struct Assets {
    texture: ugli::Texture,
    sound: geng::Sound,
}

pub struct State {
    geng: Geng,
    assets: Assets,
    framebuffer_size: Vec2<f32>,
    camera: geng::Camera2d,
    time: f32,
    pos: Vec2<f32>,
}

impl Feature for State {
    fn load(geng: Geng, path: std::path::PathBuf) -> Pin<Box<dyn Future<Output = Self>>>
    where
        Self: Sized,
    {
        async move {
            Self {
                assets: geng::LoadAsset::load(&geng, &path).await.unwrap(),
                geng,
                framebuffer_size: vec2(1.0, 1.0),
                camera: geng::Camera2d {
                    center: Vec2::ZERO,
                    rotation: 0.0,
                    fov: 10.0,
                },
                time: 0.0,
                pos: Vec2::ZERO,
            }
        }
        .boxed_local()
    }

    fn update(&mut self, delta_time: f32) {
        self.time -= delta_time;
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.time < 0.0 {
            return;
        }
        self.framebuffer_size = framebuffer.size().map(|x| x as f32);
        self.geng.draw_2d(
            framebuffer,
            &self.camera,
            &draw_2d::TexturedQuad::colored(
                AABB::point(self.pos).extend_symmetric(
                    vec2(
                        self.assets.texture.size().x as f32 / self.assets.texture.size().y as f32,
                        1.0,
                    ) * (2.0 - self.time),
                ),
                &self.assets.texture,
                Rgba::new(1.0, 1.0, 1.0, 1.0 - self.time),
            ),
        );
    }

    fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        if message.trim() != "!boom" {
            return;
        }
        self.time = 1.0;
        self.assets.sound.play();
        self.pos = vec2(
            global_rng().gen_range(-1.0..1.0) * self.framebuffer_size.x / self.framebuffer_size.y,
            global_rng().gen_range(-1.0..1.0),
        ) * self.camera.fov
            / 2.0
            * 0.75;
    }
}
