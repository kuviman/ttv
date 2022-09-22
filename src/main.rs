use geng::prelude::*;

mod app;
mod font;
mod ttv;

fn main() {
    logger::init().unwrap();

    ttv::test();
    return;

    ttv::refresh_token();
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
                move |assets| app::State::new(&geng, &Rc::new(assets.unwrap()), ttv::Client::new())
            },
        ),
    );
}
