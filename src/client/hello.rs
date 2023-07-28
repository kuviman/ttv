use super::*;

#[derive(geng::asset::Load)]
struct Assets {
    #[load(list = "1..=3", path = "*.wav")]
    sound: Vec<geng::Sound>,
    crab: ugli::Texture,
}

pub struct State {
    geng: Geng,
    camera: geng::Camera2d,
    assets: Assets,
    time: f32,
    name: String,
    font_program: ugli::Program,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, assets_path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        Self {
            assets: geng::asset::Load::load(geng.asset_manager(), &assets_path, &default())
                .await
                .unwrap(),
            font_program: geng.shader_lib().compile(font::SHADER_SOURCE).unwrap(),
            camera: geng::Camera2d {
                center: vec2::ZERO,
                rotation: Angle::ZERO,
                fov: 10.0,
            },
            geng,
            time: 10.0,
            name: "".to_owned(),
        }
    }

    async fn update(&mut self, delta_time: f32) {
        self.time += delta_time / 2.0;
    }

    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        if self.time > 2.0 {
            return;
        }
        let x =
            self.camera.fov / 2.0 * framebuffer.size().x as f32 / framebuffer.size().y as f32 - 2.0;
        let y = -self.camera.fov / 2.0 + 1.0;
        self.geng.draw2d().draw2d(
            framebuffer,
            &self.camera,
            &draw2d::TexturedQuad::new(
                Aabb2::point(vec2(x, y - 2.0 * (self.time - 1.0).abs().sqr())).extend_uniform(1.0),
                &self.assets.crab,
            ),
        );
        self.geng.draw2d().draw2d(
            framebuffer,
            &self.camera,
            &font::Text::unit(
                &self.geng,
                &self.font_program,
                &**self.geng.default_font(),
                format!("Hello, {}", self.name),
                Rgba::WHITE,
                Rgba::BLACK,
            )
            .scale_uniform(0.15)
            .translate(vec2(x, y - 0.5)),
        );
    }

    async fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::RewardRedemption { name, reward } = message else { return };
        if reward != "Hello" {
            return;
        }
        self.assets.sound.choose(&mut thread_rng()).unwrap().play();
        self.time = 0.0;
        self.name = name.clone();
    }
}
