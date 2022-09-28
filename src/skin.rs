use super::*;

#[derive(sqlx::FromRow, PartialEq, Debug)]
pub struct Skin {
    pub face: String,
    pub hat: String,
    pub robe: String,
    pub beard: String,
    #[sqlx(rename = "custom_skin")]
    pub custom: Option<String>,
    #[sqlx(try_from = "String")]
    pub outfit_color: Rgba<f32>,
}

impl ToString for Skin {
    fn to_string(&self) -> String {
        if let Some(custom) = &self.custom {
            return format!("custom({})", custom);
        }
        format!(
            "face:{}, hat:{}, robe:{}, beard:{}, color:{}",
            self.face, self.hat, self.robe, self.beard, self.outfit_color
        )
    }
}

impl Skin {
    pub fn random(assets: &app::Assets) -> Self {
        Self {
            face: assets
                .guy
                .face
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            hat: assets
                .guy
                .hat
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            robe: assets
                .guy
                .robe
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            beard: assets
                .guy
                .beard
                .keys()
                .choose(&mut global_rng())
                .unwrap()
                .clone(),
            custom: None,
            outfit_color: *assets
                .constants
                .guy_palette
                .choose(&mut global_rng())
                .unwrap(),
        }
    }
}
