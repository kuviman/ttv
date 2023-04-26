use super::*;

pub struct State {
    config: HashMap<String, String>,
    connection: Connection,
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
        Self {
            config: serde_json::from_str(&json).unwrap(),
            connection,
        }
    }

    async fn update(&mut self, _delta_time: f32) {}

    fn draw(&mut self, _framebuffer: &mut ugli::Framebuffer) {}

    async fn handle(&mut self, message: &ServerMessage) {
        let this = self;
        let ServerMessage::ChatMessage { message, .. } = message else { return };
        let Some(reply) = this.config.get(message.trim()) else { return };
        this.connection.say(reply);
    }
}
