//! Yakbak HTTP record/replay harness for chat_stream integration tests.
//!
//! Provides a local HTTP server that replays pre-recorded SSE responses,
//! enabling fully deterministic streaming tests without real LLM calls.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hyper::body::Bytes;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use tokio::sync::oneshot;

/// Replay mode configuration.
pub struct ReplayConfig {
    /// Directory containing response_NNN.txt files.
    pub cassette_dir: PathBuf,
}

/// A lightweight HTTP server that replays pre-recorded responses.
pub struct YakbakServer {
    addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
}

impl YakbakServer {
    /// Start a replay server serving cassettes from the given directory.
    pub async fn start_replay(cassette_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let cassette_dir = cassette_dir.into();
        if !cassette_dir.exists() {
            return Err(format!("Cassette directory not found: {}", cassette_dir.display()));
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let dir = Arc::new(cassette_dir);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let make_svc = make_service_fn(move |_| {
            let counter = counter.clone();
            let dir = dir.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |_req: Request<Body>| {
                    let counter = counter.clone();
                    let dir = dir.clone();
                    async move { serve_replay(&dir, &counter).await }
                }))
            }
        });

        let addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let server = Server::bind(&addr).serve(make_svc);
        let bound_addr = server.local_addr();

        let graceful = server.with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        });

        let join_handle = tokio::spawn(async move {
            if let Err(e) = graceful.await {
                eprintln!("[yakbak] server error: {e}");
            }
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

/// Serve the next cassette file as an HTTP response.
async fn serve_replay(
    cassette_dir: &Path,
    counter: &AtomicUsize,
) -> Result<Response<Body>, hyper::Error> {
    let idx = counter.fetch_add(1, Ordering::SeqCst);

    // Find all .txt files sorted
    let mut files: Vec<PathBuf> = std::fs::read_dir(cassette_dir)
        .unwrap_or_else(|_| panic!("Cannot read cassette dir: {}", cassette_dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "txt"))
        .collect();
    files.sort();

    if idx >= files.len() {
        // No more cassettes — return 500
        return Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("No more cassette files"))
            .unwrap());
    }

    let content = std::fs::read(&files[idx])
        .unwrap_or_else(|_| panic!("Cannot read cassette: {}", files[idx].display()));

    // Infer content-type from body
    let content_type = infer_content_type(&content);

    // Stream in chunks to simulate real HTTP behavior
    let chunk_size = 8192;
    let chunks: Vec<Bytes> = content
        .chunks(chunk_size)
        .map(|c| Bytes::copy_from_slice(c))
        .collect();

    let stream = futures::stream::iter(chunks.into_iter().map(Ok::<_, hyper::Error>));

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type)
        .header("transfer-encoding", "chunked")
        .body(Body::wrap_stream(stream))
        .unwrap())
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
