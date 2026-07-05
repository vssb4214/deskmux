use tokio::net::TcpListener;

use crate::config::{Config, LoadError};

use super::bind::resolve_bind_addr;
use super::handlers::AppState;

/// Starts the local HTTP API on a background thread. When config failed to load,
/// `/health` still responds but `/status` and `/apply-preset` return 503.
pub fn spawn_server(config_result: Result<Config, LoadError>) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            if let Err(err) = run_server(config_result).await {
                eprintln!("deskmux: API server error: {err}");
            }
        });
    });
}

async fn run_server(
    config_result: Result<Config, LoadError>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = resolve_bind_addr(config_result.as_ref().ok());
    let listener = TcpListener::bind(addr).await?;
    eprintln!("deskmux: API listening on http://{addr}");
    axum::serve(
        listener,
        super::router(AppState::from_load_result(config_result)),
    )
    .await?;
    Ok(())
}
