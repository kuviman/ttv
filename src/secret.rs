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

pub struct Secrets {
    path: std::path::PathBuf,
    pub config: Config,
}

impl Secrets {
    pub fn new(path: impl AsRef<std::path::Path>) -> eyre::Result<Self> {
        let path = path.as_ref().to_owned();
        let config: Config = toml::from_str(&read_file(path.join("config.toml"))?)?;
        Ok(Self { path, config })
    }
    pub fn read() -> eyre::Result<Self> {
        Self::new(static_path().join("secret"))
    }
    pub fn ttv_access_token(&self, login: impl AsRef<str>) -> eyre::Result<String> {
        let tokens_file_path = self
            .path
            .join("tokens")
            .join(format!("{}.json", login.as_ref()));
        let tokens: ttv::auth::Tokens = match std::fs::File::open(&tokens_file_path) {
            Ok(file) => {
                let tokens: ttv::auth::Tokens = serde_json::from_reader(file)?;
                if block_on(ttv::auth::validate(&tokens.access_token))? {
                    tokens
                } else {
                    block_on(ttv::auth::refresh(
                        &self.config.ttv.client_id,
                        &self.config.ttv.client_secret,
                        &tokens.refresh_token,
                    ))?
                }
            }
            Err(_) => block_on(ttv::auth::authenticate(
                &self.config.ttv.client_id,
                &self.config.ttv.client_secret,
                true,
                &["channel:read:redemptions", "chat:edit", "chat:read"].map(ttv::auth::Scope::new),
            ))?,
        };
        std::fs::create_dir_all(tokens_file_path.parent().unwrap())?;
        serde_json::to_writer_pretty(std::fs::File::create(&tokens_file_path)?, &tokens)?;
        Ok(tokens.access_token)
    }
}
