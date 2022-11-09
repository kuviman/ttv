use super::*;

#[derive(geng::Assets)]
struct Assets {
    yeti: ugli::Texture,
    #[asset(path = "JumpScare1.wav")]
    sound: geng::Sound,
}

pub struct State {
    geng: Geng,
    assets: Assets,
    time: f32,
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
                time: 0.0,
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
        self.geng.draw_2d(
            framebuffer,
            &geng::Camera2d {
                center: Vec2::ZERO,
                rotation: 0.0,
                fov: 10.0,
            },
            &draw_2d::TexturedQuad::new(
                AABB::point(Vec2::ZERO).extend_uniform(5.0),
                &self.assets.yeti,
            ),
        );
    }

    fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        if message.trim() != "!jumpscare" {
            return;
        }
        self.assets.sound.play();
        self.time = 1.0;
    }
}
