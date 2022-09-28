use geng::prelude::*;

mod app;
mod db;
mod font;
mod secret;
mod skin;
mod ttv;
mod util;

use db::Db;
use secret::Secrets;
use skin::Skin;
use util::*;

#[derive(clap::Parser)]
pub struct Opt {
    #[clap(long)]
    pub no_chat_spam: bool,
}

#[derive(Deserialize)]
pub struct Config {
    pub channel_login: String,
    pub bot_login: String,
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
                    let config: Config = serde_json::from_reader(
                        std::fs::File::open(static_path().join("config.json")).unwrap(),
                    )
                    .unwrap();
                    let ttv_client = ttv::Client::new(&config.channel_login, &config.bot_login);
                    app::State::new(&geng, &Rc::new(assets), config, ttv_client, opt)
                }
            },
        ),
    );
}
