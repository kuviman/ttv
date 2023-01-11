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
    pub fn init_from(path: impl AsRef<std::path::Path>) -> eyre::Result<Self> {
        debug!("Initializing secrets");
        let path = path.as_ref().to_owned();
        let config: Config = toml::from_str(&read_file(path.join("config.toml"))?)?;
        debug!("Secrets are all set up");
        Ok(Self { path, config })
    }
    pub fn init() -> eyre::Result<Self> {
        Self::init_from(run_dir().join("secret"))
    }
    pub fn ttv_access_token(&self, login: impl AsRef<str>) -> eyre::Result<String> {
        let login = login.as_ref();
        debug!("Getting ttv token for {:?}", login);
        let tokens_file_path = self.path.join("tokens").join(format!("{}.json", login));
        let tokens: ttv::auth::Tokens = match std::fs::File::open(&tokens_file_path) {
            Ok(file) => {
                debug!("Reading existing tokens");
                let tokens: ttv::auth::Tokens = serde_json::from_reader(file)?;
                if block_on(ttv::auth::validate(&tokens.access_token))? {
                    debug!("Token still valid");
                    tokens
                } else {
                    debug!("Token invalid, refreshing");
                    block_on(ttv::auth::refresh(
                        &self.config.ttv.client_id,
                        &self.config.ttv.client_secret,
                        &tokens.refresh_token,
                    ))?
                }
            }
            Err(_) => {
                info!("Auth not setup, prepare to login as {:?}", login);
                block_on(ttv::auth::authenticate(
                    &self.config.ttv.client_id,
                    &self.config.ttv.client_secret,
                    true,
                    &["channel:read:redemptions", "chat:edit", "chat:read"]
                        .map(ttv::auth::Scope::new),
                ))?
            }
        };
        std::fs::create_dir_all(tokens_file_path.parent().unwrap())?;
        serde_json::to_writer_pretty(std::fs::File::create(&tokens_file_path)?, &tokens)?;
        debug!("Token retrieved successfully");
        Ok(tokens.access_token)
    }
}
