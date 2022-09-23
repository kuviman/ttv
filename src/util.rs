use super::*;

pub fn block_on<F: Future>(future: F) -> F::Output {
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

pub fn read_file(path: &str) -> eyre::Result<String> {
    let mut result = String::new();
    std::fs::File::open(path)?.read_to_string(&mut result)?;
    Ok(result)
}
