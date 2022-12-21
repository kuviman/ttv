use super::*;

use reqwest::Url;

pub fn block_on<F: Future>(future: F) -> F::Output {
    if let Ok(_handle) = tokio::runtime::Handle::try_current() {
        panic!("Running blocking code in async code is bad, you know there is spawn_blocking");
    } else {
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio_runtime.block_on(future)
    }
}

pub fn read_file(path: impl AsRef<std::path::Path>) -> eyre::Result<String> {
    let mut result = String::new();
    std::fs::File::open(path)?.read_to_string(&mut result)?;
    Ok(result)
}

/// Run a local server and wait for an http request, and return the uri
pub async fn wait_for_request_uri() -> eyre::Result<Url> {
    let addr: std::net::SocketAddr = "127.0.0.1:3000".parse().unwrap();
    debug!("Listening {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    // We just wait for the first connection
    debug!("Waiting for connection...");
    let (stream, _) = listener.accept().await?;
    debug!("Got connection");

    // Use a channel because how else?
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<hyper::Uri>();
    // We could use oneshot channel but this service fn must be impl FnMut
    // because there may be multiple request even on single connection?
    let service = |request: hyper::Request<hyper::Body>| {
        let sender = sender.clone();
        async move {
            sender.send(request.uri().clone())?;
            Ok::<_, eyre::Report>(hyper::Response::new(hyper::Body::from(
                "You may now close this tab",
            )))
        }
    };
    // No keepalive so we return immediately
    hyper::server::conn::Http::new()
        .http1_keep_alive(false)
        .http2_keep_alive_interval(None)
        .serve_connection(stream, hyper::service::service_fn(service))
        .await?;
    let uri = receiver
        .recv()
        .await
        .ok_or_else(|| eyre::Report::msg("Failed to wait for the request"))?;
    Ok(Url::parse("http://localhost:3000")
        .unwrap()
        .join(&uri.path_and_query().unwrap().as_str())?)
}
