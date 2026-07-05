use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::{router, AppState};

/// Binds an ephemeral loopback port and serves the DeskMux API router.
pub async fn spawn_test_server(state: AppState) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral loopback port");
    spawn_test_server_on(listener, state).await
}

/// Serves the API on an already-bound listener (used when both ports must be known upfront).
pub async fn spawn_test_server_on(
    listener: TcpListener,
    state: AppState,
) -> (SocketAddr, JoinHandle<()>) {
    let addr = listener.local_addr().expect("local addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, router(state))
            .await
            .expect("serve test API");
    });
    (addr, handle)
}
