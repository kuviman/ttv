use super::*;

pub struct Db {
    connection: Connection,
}

impl Db {
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub async fn find_level(&self, name: &str) -> usize {
        self.connection
            .get_key_value(&format!("raffle_royale/{name}/level"))
            .await
            .unwrap_or(1)
    }

    pub fn set_level(&self, name: &str, level: usize) {
        self.connection
            .set_key_value(&format!("raffle_royale/{name}/level"), &level)
    }

    pub async fn find_skin(&self, name: &str) -> Option<Skin> {
        self.connection
            .get_key_value(&format!("raffle_royale/{name}/skin"))
            .await
    }

    pub fn set_skin(&self, name: &str, skin: &Skin) {
        self.connection
            .set_key_value(&format!("raffle_royale/{name}/skin"), skin)
    }

    pub async fn find_game_link(&self, name: &str) -> Option<String> {
        self.connection
            .get_key_value::<Option<String>>(&format!("raffle_royale/{name}/game_link"))
            .await
            .flatten()
    }

    pub fn set_game_link(&self, name: &str, url: Option<&str>) {
        self.connection
            .set_key_value(&format!("raffle_royale/{name}/game_link"), &url)
    }

    pub async fn game_played(&self, name: &str) -> bool {
        self.connection
            .get_key_value(&format!("raffle_royale/{name}/game_played"))
            .await
            .unwrap_or(false)
    }
}
