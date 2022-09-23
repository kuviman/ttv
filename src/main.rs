use geng::prelude::*;

mod api;
mod app;
mod db;
mod font;

use db::Db;

fn block_on<F: Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(future)
    } else {
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio_runtime.block_on(future)
    }
}

fn main() {
    logger::init().unwrap();

    api::refresh_token();
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
                    app::State::new(&geng, &Rc::new(assets), api::Client::new())
                }
            },
        ),
    );
}
