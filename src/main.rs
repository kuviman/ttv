use geng::prelude::*;

mod app;
mod db;
mod font;
mod secret;
mod ttv;
mod util;

use db::Db;
use util::*;

fn main() {
    logger::init().unwrap();

    let geng = Geng::new("ttv");
    let geng = &geng;
    geng::run(
        geng,
        geng::LoadingScreen::new(
            geng,
            geng::EmptyLoadingScreen,
            <app::Assets as geng::LoadAsset>::load(geng, &static_path()),
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
