use super::*;

#[derive(Serialize, Deserialize)]
pub struct DbConfig {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct TtvConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub db: DbConfig,
    pub ttv: TtvConfig,
}

impl Config {
    pub fn read() -> eyre::Result<Self> {
        Ok(toml::from_str(&read_file("secret/config.toml")?)?)
    }
}
