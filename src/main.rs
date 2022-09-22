use geng::prelude::*;

mod app;
mod font;
mod ttv;

fn main() {
    logger::init().unwrap();

    // ttv::refresh_token();

    // return;

    // let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
    //     .enable_all()
    //     .build()
    //     .unwrap();
    // let user_id = tokio_runtime.block_on(async {
    //     info!("Sending request");
    //     let response = reqwest::get("https://api.twitch.tv/helix/users")
    //         .await
    //         .unwrap();
    //     info!("Got response");
    //     let json = response.text().await.unwrap();
    //     info!("{:?}", json);
    // });

    // let mut ws = websocket_lite::ClientBuilder::new("wss://pubsub-edge.twitch.tv")
    //     .unwrap()
    //     .connect()
    //     .unwrap();

    // let mut token = String::new();
    // std::fs::File::open("secret/token")
    //     .unwrap()
    //     .read_to_string(&mut token)
    //     .unwrap();
    // token = token.trim().to_owned();
    // let request = serde_json::json!({
    //     "type": "LISTEN",
    //     "nonce": "kekw",
    //     "data": {
    //         "topics": ["channel-points-channel-v1.594062839"],
    //         "auth_token": token,
    //     }
    // });
    // ws.send(websocket_lite::Message::text(
    //     serde_json::to_string(&request).unwrap(),
    // ))
    // .unwrap();
    // while let Ok(Some(message)) = ws.receive() {
    //     info!("{:?}", message);
    // }

    // return;
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
