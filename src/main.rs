use geng::prelude::*;

mod app;
mod db;
mod font;
mod secret;
mod ttv;
mod util;

use db::Db;
use secret::Secrets;
use util::*;

#[derive(sqlx::FromRow, PartialEq, Debug)]
pub struct Skin {
    face: String,
    hat: String,
    robe: String,
    beard: String,
    #[sqlx(rename = "custom_skin")]
    custom: Option<String>,
    #[sqlx(try_from = "String")]
    outfit_color: Rgba<f32>,
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
            outfit_color: *assets.config.guy_palette.choose(&mut global_rng()).unwrap(),
        }
    }
}

#[derive(clap::Parser)]
pub struct Opt {
    #[clap(long)]
    pub no_chat_spam: bool,
}

fn main() {
    {
        let mut builder = logger::builder();
        builder.parse_filters("geng=info");
        builder.parse_filters("sqlx=off");
        builder.parse_filters("reqwest=off");
        logger::init_with(builder).unwrap();
    }

    let opt: Opt = program_args::parse();

    let geng = Geng::new("ttv");
    let geng = &geng;
    geng::run(
        geng,
        geng::LoadingScreen::new(
            geng,
            geng::EmptyLoadingScreen,
            <app::Assets as geng::LoadAsset>::load(geng, &static_path().join("assets")),
            {
                let geng = geng.clone();
                move |assets| {
                    let mut assets = assets.unwrap();
                    assets.process();
                    app::State::new(&geng, &Rc::new(assets), ttv::Client::new(), opt)
                }
            },
        ),
    );
}
