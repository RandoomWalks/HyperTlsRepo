use hyper::{service::service_fn, server::conn::Http, Body, Request, Response};
use rustls::{Certificate, PrivateKey, ServerConfig};
use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

mod load_cfg;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS configuration
    let tls_config = Arc::new(load_tls_config()?);
    let tls_acceptor = TlsAcceptor::from(tls_config);

    // Bind the server to a port
    let listener = TcpListener::bind("127.0.0.1:8443").await?;
    println!("Proxy server listening on https://127.0.0.1:8443");

    loop {
        let (stream, _) = listener.accept().await?;
        let tls_acceptor = tls_acceptor.clone();

        // Handle each connection in a new task
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, tls_acceptor).await {
                eprintln!("Error handling connection: {:?}", err);
            }
        });
    }
}

/// Handles an incoming TLS connection and serves HTTP requests.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    tls_acceptor: TlsAcceptor,
) -> Result<(), Box<dyn std::error::Error>> {
    // Record the start of the connection for diagnostics
    let connection_id = Uuid::new_v4();
    let created_at = Instant::now();
    println!("[INFO] Connection {} started", connection_id);

    // Accept the TLS handshake
    let tls_stream = tls_acceptor.accept(stream).await?;
    println!("[INFO] Connection {}: TLS handshake complete", connection_id);

    // Serve HTTP requests over the TLS stream
    Http::new()
        .serve_connection(
            tls_stream,
            service_fn(move |req| handle_request(req, connection_id)),
        )
        .await?;

    // Log diagnostics after connection closes
    println!(
        "[INFO] Connection {} closed. Duration: {:?}",
        connection_id,
        created_at.elapsed()
    );

    Ok(())
}

/// Simple request handler that simulates proxy behavior.
async fn handle_request(
    req: Request<Body>,
    connection_id: Uuid,
) -> Result<Response<Body>, hyper::Error> {
    println!("[INFO] Connection {} received request: {:?}", connection_id, req);
    Ok(Response::new(Body::from("Hello from the proxy server!")))
}
