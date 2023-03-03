use geng::prelude::*;

mod client;
#[cfg(not(target_arch = "wasm32"))]
mod server;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    ChatMessage {
        name: String,
        message: String,
    },
    RewardRedemption {
        name: String,
        reward: String,
    },
    KeyValue {
        request_id: String,
        value: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    Say { text: String },
    GetKeyValue { request_id: String, key: String },
    SetKeyValue { key: String, value: String },
}

#[derive(clap::Parser)]
struct Opt {
    #[clap(long)]
    pub server: Option<String>,
    #[clap(long)]
    pub connect: Option<String>,
    #[clap(long)]
    pub serve: Option<std::path::PathBuf>,
}

fn main() {
    {
        let mut builder = logger::builder();
        builder.parse_filters("geng=info");
        builder.parse_filters("reqwest=off");
        logger::init_with(builder).unwrap();
    }
    geng::setup_panic_handler();

    let mut opt: Opt = program_args::parse();

    if opt.connect.is_none() && opt.server.is_none() {
        if cfg!(target_arch = "wasm32") {
            opt.connect = Some("ws://127.0.0.1:1155".to_owned());
        } else {
            opt.server = Some("127.0.0.1:1155".to_owned());
            opt.connect = Some("ws://127.0.0.1:1155".to_owned());
        }
    }

    if opt.server.is_some() && opt.connect.is_none() {
        #[cfg(not(target_arch = "wasm32"))]
        server::run(opt.server.as_deref().unwrap(), opt.serve.as_deref());
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(addr) = &opt.server {
            let addr = addr.to_owned();
            std::thread::spawn(move || {
                server::run(&addr, opt.serve.as_deref());
            });
        }
        client::run(opt.connect.as_deref().unwrap());
    }
}
