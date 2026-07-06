//! The serving machinery: bind the endpoint, accept connections, and dispatch
//! each request through the router with a per-request context. The context
//! carries the connection's outbound channel, so streaming channels
//! (`events.subscribe`) are ordinary handlers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ipc::protocol::{decode_request, encode_response, Response};
use ipc::{Connection, Peer};
use tracing::Instrument;

use crate::runtime::{HandlerContext, Router, Runtime};
use crate::services::make_router;

pub async fn run_daemon(log_path: std::path::PathBuf) -> i32 {
    let endpoint = ipc::endpoint::default_endpoint();
    let listener = match ipc::bind(&endpoint).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("cannot start: {e}");
            return 1;
        }
    };

    let runtime = Arc::new(Runtime::new(log_path, None));
    tracing::info!(
        version = common::app::VERSION,
        pid = std::process::id(),
        home = %runtime.engine().data_home().display(),
        "hestiad starting"
    );
    runtime.processes().recover();
    let router = Arc::new(make_router());

    tracing::info!("hestiad listening on {}", endpoint.display());
    tokio::select! {
        _ = accept_loop(listener, router, runtime.clone()) => {}
        _ = runtime.stopped() => tracing::info!("stop requested"),
        _ = shutdown_signal() => tracing::info!("signal received"),
    }
    runtime.shutdown_workloads().await;
    tracing::info!("hestiad stopped");
    0
}

async fn accept_loop(listener: ipc::Listener, router: Arc<Router>, runtime: Arc<Runtime>) {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    loop {
        let (conn, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::debug!("accept failed: {e}");
                continue;
            }
        };
        if !peer.authorized() {
            tracing::warn!(
                uid = peer.uid,
                "rejecting connection from unauthorized peer"
            );
            continue;
        }
        let router = router.clone();
        let runtime = runtime.clone();
        let conn_id = COUNTER.fetch_add(1, Ordering::Relaxed);
        // A span per connection, so every request and handler log it fans out is
        // tagged with the connection id and peer uid — the traceability seam.
        let span = tracing::info_span!("conn", id = conn_id, uid = peer.uid);
        tokio::spawn(
            async move {
                tracing::debug!("client connected");
                serve_connection(conn, peer, conn_id, router, runtime).await;
                tracing::debug!("client disconnected");
            }
            .instrument(span),
        );
    }
}

async fn serve_connection(
    conn: Connection,
    peer: Peer,
    conn_id: u64,
    router: Arc<Router>,
    runtime: Arc<Runtime>,
) {
    let (mut reader, mut writer) = conn.into_split();
    let (out_tx, mut out_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            if writer.send(&frame).await.is_err() {
                break;
            }
        }
    });

    while let Ok(Some(frame)) = reader.recv().await {
        let req = match decode_request(&frame) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("dropping malformed frame: {e}");
                let _ = out_tx.send(encode_response(&Response::failure(
                    ipc::errors::BAD_REQUEST,
                    e.to_string(),
                )));
                continue;
            }
        };
        let id = req.id;
        let ctx = HandlerContext {
            runtime: runtime.clone(),
            conn_id,
            out: out_tx.clone(),
            peer,
        };
        let mut res = router.route(req, ctx).await;
        res.id = id;
        let _ = out_tx.send(encode_response(&res));
    }

    runtime.hub().unsubscribe(conn_id);
    drop(out_tx);
    let _ = writer_task.await;
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
