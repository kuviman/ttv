use super::*;

pub struct State {
    sounds: HashMap<String, geng::Sound>,
}

#[async_trait(?Send)]
impl Feature for State {
    async fn load(geng: Geng, path: std::path::PathBuf, connection: Connection) -> Self
    where
        Self: Sized,
    {
        let json: String = geng::asset::Load::load(&geng, &path.join("config.json"))
            .await
            .unwrap();
        let list: Vec<String> = serde_json::from_str(&json).unwrap();
        let sounds = future::join_all(list.into_iter().map(|name| {
            let geng = geng.clone();
            let path = path.clone();
            async move {
                (
                    format!("!{name}"),
                    <geng::Sound as geng::asset::Load>::load(
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

    async fn update(&mut self, _delta_time: f32) {}

    fn draw(&mut self, _framebuffer: &mut ugli::Framebuffer) {}

    async fn handle(&mut self, message: &ServerMessage) {
        let this = self;
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        let Some(sound) = this.sounds.get(message.trim()) else { return };
        sound.play();
    }
}
