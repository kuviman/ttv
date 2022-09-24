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
        Ok(toml::from_str(&read_file(
            static_path().join("secret").join("config.toml"),
        )?)?)
    }
}

pub fn get_ttv_access_token(login: impl AsRef<str>) -> eyre::Result<String> {
    let secret_config = Config::read()?;
    let tokens_file_path = static_path()
        .join("secret")
        .join("tokens")
        .join(format!("{}.json", login.as_ref()));
    let tokens: ttv::auth::Tokens = match std::fs::File::open(&tokens_file_path) {
        Ok(file) => {
            let tokens: ttv::auth::Tokens = serde_json::from_reader(file)?;
            // TODO: ttv::auth::validate to maybe just use current token
            block_on(ttv::auth::refresh(
                &secret_config.ttv.client_id,
                &secret_config.ttv.client_secret,
                &tokens.refresh_token,
            ))?
        }
        Err(_) => block_on(ttv::auth::authenticate(
            &secret_config.ttv.client_id,
            &secret_config.ttv.client_secret,
            true,
            &["channel:read:redemptions", "chat:edit", "chat:read"].map(ttv::auth::Scope::new),
        ))?,
    };
    std::fs::create_dir_all(tokens_file_path.parent().unwrap())?;
    serde_json::to_writer_pretty(std::fs::File::create(&tokens_file_path)?, &tokens)?;
    Ok(tokens.access_token)
}
