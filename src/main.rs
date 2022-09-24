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

fn main() {
    {
        let mut builder = logger::builder();
        builder.parse_filters("geng=info");
        builder.parse_filters("sqlx=off");
        builder.parse_filters("reqwest=off");
        logger::init_with(builder).unwrap();
    }

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
                    app::State::new(&geng, &Rc::new(assets), ttv::Client::new())
                }
            },
        ),
    );
}
