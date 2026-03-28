use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::response::IntoResponse;
use axum::routing::get;
use futures::stream;
use tokio::io::AsyncReadExt as _;
use tokio::io::AsyncWriteExt as _;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    let (drop_tx, drop_rx) = oneshot::channel();
    let emitted_chunks = Arc::new(AtomicUsize::new(0));
    let app = app(drop_tx, emitted_chunks.clone());
    let (addr, server_task) = spawn_server(app).await;

    let mut client = TcpStream::connect(addr).await.expect("connect");
    client
        .write_all(b"GET /stream HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .expect("write request");

             sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
            sleep(Duration::from_millis(100)).await;
   let mut response = [0_u8; 512];
    let count = client.read(&mut response).await.expect("read response");
    println!("read {count} bytes from server, then closing client socket");

    drop(client);

    match timeout(Duration::from_secs(3), drop_rx).await {
        Ok(Ok(())) => {
            println!(
                "stream dropped after disconnect; emitted {} chunks",
                emitted_chunks.load(Ordering::SeqCst)
            );
        }
        Ok(Err(_)) => {
            eprintln!("drop notification channel closed unexpectedly");
            std::process::exit(2);
        }
        Err(_) => {
            eprintln!(
                "timeout waiting for stream drop after disconnect; emitted {} chunks",
                emitted_chunks.load(Ordering::SeqCst)
            );
            std::process::exit(1);
        }
    }

    server_task.abort();
}

fn app(drop_tx: oneshot::Sender<()>, emitted_chunks: Arc<AtomicUsize>) -> Router {
    let drop_tx = Arc::new(Mutex::new(Some(drop_tx)));
    Router::new().route(
        "/stream",
        get(move || {
            let drop_tx = drop_tx
                .lock()
                .expect("lock drop sender")
                .take()
                .expect("single request");
            let emitted_chunks = emitted_chunks.clone();
            async move { stream_response(drop_tx, emitted_chunks) }
        }),
    )
}

fn stream_response(
    drop_tx: oneshot::Sender<()>,
    emitted_chunks: Arc<AtomicUsize>,
) -> impl IntoResponse {
    struct DropNotice {
        tx: Option<oneshot::Sender<()>>,
    }

    impl Drop for DropNotice {
        fn drop(&mut self) {
            if let Some(tx) = self.tx.take() {
                let _ = tx.send(());
            }
        }
    }

    let guard = DropNotice { tx: Some(drop_tx) };
    let stream = stream::unfold((0_usize, Some(guard)), move |(index, guard)| {
        let emitted_chunks = emitted_chunks.clone();
        async move {
            sleep(Duration::from_millis(100)).await;
            emitted_chunks.store(index + 1, Ordering::SeqCst);
            let chunk = format!("chunk-{index}\n");
            Some((Ok::<_, Infallible>(chunk), (index + 1, guard)))
        }
    });

    Body::from_stream(stream)
}

async fn spawn_server(app: Router) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let server = async move {
        axum::serve(listener, app).await.expect("serve");
    };
    (addr, tokio::spawn(server))
}
