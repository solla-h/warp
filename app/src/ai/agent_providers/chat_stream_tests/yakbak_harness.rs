//! Yakbak HTTP record/replay harness for chat_stream integration tests.
//!
//! Provides a local HTTP server that replays pre-recorded SSE responses,
//! enabling fully deterministic streaming tests without real LLM calls.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// A lightweight HTTP server that replays pre-recorded responses.
pub struct YakbakServer {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Clone)]
struct ReplayState {
    counter: Arc<AtomicUsize>,
    cassette_dir: Arc<PathBuf>,
}

impl YakbakServer {
    /// Start a replay server serving cassettes from the given directory.
    pub async fn start_replay(cassette_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let cassette_dir = cassette_dir.into();
        if !cassette_dir.exists() {
            return Err(format!(
                "Cassette directory not found: {}",
                cassette_dir.display()
            ));
        }

        let state = ReplayState {
            counter: Arc::new(AtomicUsize::new(0)),
            cassette_dir: Arc::new(cassette_dir),
        };

        let app = Router::new()
            .fallback(any(handle_replay))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind: {e}"))?;
        let bound_addr = listener
            .local_addr()
            .map_err(|e| format!("Failed to get local addr: {e}"))?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let join_handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .ok();
        });

        Ok(Self {
            addr: bound_addr,
            shutdown_tx: Some(shutdown_tx),
            join_handle: Some(join_handle),
        })
    }

    /// The base URL of the replay server (e.g., "http://127.0.0.1:PORT/").
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.addr.port())
    }

    /// Shut down the server gracefully.
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for YakbakServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Handler that serves the next cassette file.
async fn handle_replay(State(state): State<ReplayState>) -> Response {
    let idx = state.counter.fetch_add(1, Ordering::SeqCst);

    // Find all .txt files sorted
    let mut files: Vec<PathBuf> = match std::fs::read_dir(state.cassette_dir.as_ref()) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "txt"))
            .collect(),
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cannot read cassette dir").into_response();
        }
    };
    files.sort();

    if idx >= files.len() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "No more cassette files").into_response();
    }

    let content = match std::fs::read(&files[idx]) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Cannot read cassette file").into_response();
        }
    };

    let content_type = infer_content_type(&content);

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .header("transfer-encoding", "chunked")
        .body(Body::from(content))
        .unwrap()
        .into_response()
}

fn infer_content_type(body: &[u8]) -> &'static str {
    let prefix = std::str::from_utf8(&body[..body.len().min(64)]).unwrap_or("");
    if prefix.starts_with("event:") || prefix.starts_with("data:") {
        "text/event-stream"
    } else if prefix.starts_with('{') || prefix.starts_with('[') {
        "application/json"
    } else {
        "text/plain"
    }
}

/// Convenience: start a replay server for a named fixture scenario.
///
/// Fixture files live under `app/src/ai/agent_providers/chat_stream_tests/fixtures/{provider}/{scenario}/`.
pub async fn start_fixture_replay(provider: &str, scenario: &str) -> Result<YakbakServer, String> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cassette_dir = PathBuf::from(manifest_dir)
        .join("src/ai/agent_providers/chat_stream_tests/fixtures")
        .join(provider)
        .join(scenario);
    YakbakServer::start_replay(cassette_dir).await
}
