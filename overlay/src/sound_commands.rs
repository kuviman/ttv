use super::*;

pub struct State {
    sounds: HashMap<String, geng::Sound>,
}

impl Feature for State {
    fn load(geng: Geng, path: std::path::PathBuf) -> Pin<Box<dyn Future<Output = Self>>>
    where
        Self: Sized,
    {
        async move {
            let json: String = geng::LoadAsset::load(&geng, &path.join("config.json"))
                .await
                .unwrap();
            let list: Vec<String> = serde_json::from_str(&json).unwrap();
            let sounds = future::join_all(list.into_iter().map(|name| {
                let geng = geng.clone();
                let path = path.clone();
                async move {
                    (
                        format!("!{name}"),
                        <geng::Sound as geng::LoadAsset>::load(
                            &geng,
                            &path.join(format!("{name}.wav")),
                        )
                        .await
                        .unwrap(),
                    )
                }
            }))
            .await
            .into_iter()
            .collect();
            Self { sounds }
        }
        .boxed_local()
    }

    fn update(&mut self, _delta_time: f32) {}

    fn draw(&mut self, _framebuffer: &mut ugli::Framebuffer) {}

    fn handle(&mut self, message: &ServerMessage) {
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        let Some(sound) = self.sounds.get(message.trim()) else { return };
        sound.play();
    }
}
