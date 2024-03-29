use super::*;

#[derive(geng::asset::Load)]
struct Assets {
    yeti: ugli::Texture,
    #[load(path = "JumpScare1.wav")]
    sound: geng::Sound,
}

pub struct State {
    geng: Geng,
    assets: Assets,
    time: f32,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            assets: geng::asset::Load::load(geng.asset_manager(), &path, &default())
                .await
                .unwrap(),
            geng,
            time: 0.0,
        }
    }

    async fn update(&mut self, delta_time: f32) {
        self.time -= delta_time;
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.time <= 0.0 {
            return;
        }
        self.geng.draw2d().draw2d(
            framebuffer,
            &geng::Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 10.0,
            },
            &draw2d::TexturedQuad::new(
                Aabb2::point(vec2::ZERO).extend_uniform(5.0),
                &self.assets.yeti,
            ),
        );
    }

    async fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        if message.trim() != "!jumpscare" {
            return;
        }
        self.assets.sound.play();
        self.time = 1.0;
    }
}
