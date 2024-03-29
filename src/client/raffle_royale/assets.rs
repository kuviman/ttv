use super::*;

#[derive(Deserialize, geng::asset::Load)]
#[load(serde = "json")]
pub struct Constants {
    pub channel_point_levels: usize,
    pub background: Rgba<f32>,
    pub circle: Rgba<f32>,
    pub initial_health: usize,
    pub extra_health_per_level: usize,
    pub health_per_click: usize,
    pub volume: f64,
    pub guy_palette: Vec<Rgba<f32>>,
    pub background_palette: Vec<Rgba<f32>>,
    pub beard_color: Rgba<f32>,
    pub bless_hp: usize,
    pub preferred_distance_spring_k: f32,
}

#[derive(Deref)]
pub struct Texture(#[deref] ugli::Texture);

impl std::borrow::Borrow<ugli::Texture> for Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}
impl std::borrow::Borrow<ugli::Texture> for &'_ Texture {
    fn borrow(&self) -> &ugli::Texture {
        &self.0
    }
}

impl geng::asset::Load for Texture {
    type Options = ();
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        options: &(),
    ) -> geng::asset::Future<Self> {
        let texture = <ugli::Texture as geng::asset::Load>::load(manager, path, &default());
        async move {
            let mut texture = texture.await?;
            texture.set_filter(ugli::Filter::Nearest);
            Ok(Texture(texture))
        }
        .boxed_local()
    }

    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

pub struct GuyAssets {
    pub hat: HashMap<String, Texture>,
    pub face: HashMap<String, Texture>,
    pub robe: HashMap<String, Texture>,
    pub beard: HashMap<String, Texture>,
    pub custom: HashMap<String, Texture>,
    pub custom_map: HashMap<String, String>,
}

impl geng::asset::Load for GuyAssets {
    type Options = ();
    fn load(
        manager: &geng::asset::Manager,
        path: &std::path::Path,
        _options: &(),
    ) -> geng::asset::Future<Self> {
        let manager = manager.clone();
        let path = path.to_owned();
        async move {
            let json =
                <String as geng::asset::Load>::load(&manager, &path.join("_list.json"), &default())
                    .await
                    .context("Failed to load config")?;
            #[derive(Deserialize)]
            struct Config {
                hat: Vec<String>,
                face: Vec<String>,
                robe: Vec<String>,
                beard: Vec<String>,
                custom: HashMap<String, String>,
            }
            let config: Config = serde_json::from_str(&json)?;
            let manager = &manager;
            let path = &path;
            let load_map = |class: String, list: Vec<String>| async move {
                Ok::<_, anyhow::Error>(
                    future::join_all(list.iter().map(move |name| {
                        <Texture as geng::asset::Load>::load(
                            &manager,
                            &path.join(&class).join(format!("{}.png", name)),
                            &default(),
                        )
                        .map(move |texture| (name, texture))
                    }))
                    .await
                    .into_iter()
                    .map(move |(name, texture)| {
                        texture.map(|texture| (name.clone(), texture)).unwrap()
                    })
                    .collect::<HashMap<String, Texture>>(),
                )
            };
            Ok(Self {
                hat: load_map("hat".to_owned(), config.hat)
                    .await
                    .context("Failed to load outfits")?,
                robe: load_map("robe".to_owned(), config.robe)
                    .await
                    .context("Failed to load outfits")?,
                face: load_map("face".to_owned(), config.face)
                    .await
                    .context("Failed to load faces")?,
                beard: load_map("beard".to_owned(), config.beard)
                    .await
                    .context("Failed to load outfits")?,
                custom: load_map(
                    "custom".to_owned(),
                    config.custom.values().cloned().collect(),
                )
                .await
                .context("Failed to load outfits")?,
                custom_map: config.custom,
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = None;
}

#[derive(geng::asset::Load)]
pub struct Assets {
    pub fireball: Texture,
    pub background: ugli::Texture,
    #[load(list = "1..=1", path = "background_entities/*.png")]
    pub background_entities: Vec<Texture>,
    pub constants: Constants,
    #[load(path = "kuvimanPreBattle.wav")]
    pub lobby_music: geng::Sound,
    #[load(path = "kuvimanBattle.wav")]
    pub battle_music: geng::Sound,
    #[load(list = "1..=3", path = "player_joined*.mp3")]
    pub spawn_sfx: Vec<geng::Sound>,
    #[load(path = "death.wav")]
    pub death_sfx: geng::Sound,
    #[load(path = "victory.mp3")]
    pub win_sfx: geng::Sound,
    #[load(path = "RaffleRoyaleTitle.wav")]
    pub title_sfx: geng::Sound,
    #[load(path = "levelup.wav")]
    pub levelup_sfx: geng::Sound,
    pub levelup: Rc<Texture>,
    pub levelup_front: Rc<Texture>,
    pub skull: Rc<Texture>,
    pub guy: GuyAssets,
}

impl Assets {
    pub fn process(&mut self) {
        self.lobby_music.set_looped(true);
        self.battle_music.set_looped(true);
        self.background.set_filter(ugli::Filter::Nearest);
        self.background.set_wrap_mode(ugli::WrapMode::Repeat);
    }
}
