use hyper::{
    body::Incoming,
    server::conn::http1,
    service::service_fn,
    Request, Response,
};
use std::{sync::Arc, time::Instant};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use uuid::Uuid;
use hyper_util::rt::TokioIo;

mod tls_config {
    use rustls_pki_types::{CertificateDer, PrivateKeyDer};
    use rustls::ServerConfig;
    use std::fs::File;
    use std::io::{self, BufReader};
    use rustls_pemfile::{certs, pkcs8_private_keys};

    /// Loads the TLS configuration, including certificate and private key.
    /// Assumes `cert.pem` and `key.pem` are in the current directory.
    /// - Certificates are PEM-encoded and converted to rustls::CertificateDer.
    /// - Private keys are expected in PKCS#8 format.
    /// Returns an `io::Result<ServerConfig>` which can be used to initialize the TLS server.
    pub fn load_tls_config() -> io::Result<ServerConfig> {
        let cert_file = &mut BufReader::new(File::open("cert.pem")?);
        let key_file = &mut BufReader::new(File::open("key.pem")?);

        let cert_chain: Vec<CertificateDer<'static>> = certs(cert_file)
            .collect::<Result<_, _>>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid certificate: {}", e)))?;

        // Handle PKCS8 keys specifically and convert to PrivateKeyDer
        let key = pkcs8_private_keys(key_file)
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No private keys found"))??;

        let key = PrivateKeyDer::Pkcs8(key); // Convert to PrivateKeyDer

        ServerConfig::builder()
            .with_no_client_auth() // Disables client authentication for simplicity.
            .with_single_cert(cert_chain, key)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("TLS config error: {}", e)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load TLS configuration from certificates and private key files.
    let tls_config = Arc::new(tls_config::load_tls_config()?);
    let tls_acceptor = TlsAcceptor::from(tls_config);

    // TcpListener binds to a local address and listens for incoming connections.
    // This server is designed to run on `127.0.0.1` for local testing purposes.
    // Update the bind address to `0.0.0.0` for external access in production.
    let listener = TcpListener::bind("127.0.0.1:8443").await?;
    println!("Proxy server listening on https://127.0.0.1:8443");

    // Accept and handle connections in an infinite loop.
    // Each connection is handled in its own asynchronous task using tokio::spawn.
    // This ensures non-blocking behavior but may lead to resource exhaustion under heavy load.
    // Consider implementing connection limits or pooling to optimize resource usage in production.
    while let Ok((stream, _)) = listener.accept().await {
        let tls_acceptor = tls_acceptor.clone();

        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, tls_acceptor).await {
                eprintln!("Error handling connection: {:?}", err);
            }
        });
    }

    Ok(())
}

/// Handles an individual client connection, performing a TLS handshake and serving HTTP/1 requests.
/// - Each connection is assigned a unique UUID for tracking.
/// - The function is asynchronous and non-blocking, allowing multiple connections to be handled concurrently.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    tls_acceptor: TlsAcceptor,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection_id = Uuid::new_v4();
    let created_at = Instant::now();
    println!("[INFO] Connection {} started", connection_id);

    // Perform a TLS handshake with the client.
    // The handshake might fail if the client doesn't support the expected TLS version or provides an invalid certificate.
    // Logging the error ensures the issue is recorded without crashing the server.
    let tls_stream = tls_acceptor.accept(stream).await?;
    println!("[INFO] Connection {}: TLS handshake complete", connection_id);

    // Wrap the TLS stream for use with Hyper's HTTP/1 server.
    let io = TokioIo::new(tls_stream);

    // Serve the connection using Hyper's HTTP/1 builder.
    // This is where the HTTP request is processed and the response is generated.
    http1::Builder::new()
        .serve_connection(
            io,
            service_fn(move |req: Request<Incoming>| handle_request(req, connection_id)),
        )
        .await?;

    // Log the connection duration upon closure.
    println!(
        "[INFO] Connection {} closed. Duration: {:?}",
        connection_id,
        created_at.elapsed()
    );

    Ok(())
}

/// Handles incoming HTTP requests for a single connection.
/// - Logs the request details for debugging purposes.
/// - Currently returns a static "Hello from the proxy server!" response.
/// - Future extension: This function can be enhanced to forward requests to a backend server,
/// enabling this proxy to act as a reverse proxy.
async fn handle_request(
    req: Request<Incoming>,
    connection_id: Uuid,
) -> Result<Response<String>, hyper::Error> {
    println!("[INFO] Connection {} received request: {:?}", connection_id, req);
    Ok(Response::new(String::from("Hello from the proxy server!")))
}
