use std::error::Error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
// use hyper::body::Empty;
use hyper::client::conn::http1::Builder;
use hyper::Uri;
use hyper_util::rt::TokioExecutor;
use hyper_util::server::conn::auto::Builder as ServerBuilder;
use rustls_pemfile::Item;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;
// use hyper_util::server::service_fn;
use http_body_util::Full;
use hyper::body::Body;
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper_util::client::legacy::{connect::HttpConnector, Client};
use http_body_util::Empty;

// Core utility to adapt hyper's body structure
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::task::JoinSet;

// Function to handle incoming HTTP requests. Designed for echoing the incoming payload.
// Can be adapted for more complex logic later.
async fn echo(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, std::io::Error> {
    Ok(Response::new(Full::new(Bytes::new())))
}

fn error(err: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

// Helper function to load certificates from a PEM file. Returns the DER representation of the certificates.
fn load_certs(filename: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    // Open certificate file.
    let certfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file. Ensures secure handling of the server's identity.
fn load_private_key(filename: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}


// Integration test to validate Rustls-based client-server interaction over HTTPS.
#[tokio::test]
async fn rustls_test() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Define localhost address and dynamically allocate an available port.
    let localhost = "127.0.0.1".to_owned();
    let listener = TcpListener::bind(format!("{}:0", localhost)).await?;
    let addr = listener.local_addr()?;

    // A task management structure to concurrently handle multiple asynchronous tasks.
    let mut tasks = tokio::task::JoinSet::new();

    // Task 1: Client-side logic.
    tasks.spawn(async move {
        // Configure Rustls client with root certificates loaded from PEM.
        let cert = load_certs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/common/certs/sample.pem"
        ))
        .unwrap();
        let mut roots = rustls::RootCertStore::empty();

        // Add certificates to the root certificate store.
        roots.add_parsable_certificates(cert);
        let tls = rustls::ClientConfig::builder()
            .with_root_certificates(roots) // Establish trust with the server's certificate.
            .with_no_client_auth();        // No client-side authentication required.

        // Build an HTTPS connector using the client configuration.
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http() // Support both HTTPS and HTTP.
            .enable_http1()  // Enable HTTP/1.1 protocol for compatibility.
            .build();

        // Create an HTTP client with the constructed HTTPS connector.
        let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
        let uri = Uri::from_str(format!("https://localhost:{}", addr.port()).as_str()).unwrap();

        // Perform a GET request to the server and validate the response.
        let response = client.get(uri).await.unwrap();
        assert_eq!(response.status(), 200); // Verify the server responds with HTTP 200.
        println!("client connected!");
    });

    // Task 2: Server-side logic.
    tasks.spawn(async move {
        // Load server-side certificate and private key for TLS configuration.
        let cert = load_certs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/common/certs/sample.pem"
        ))
        .unwrap();
        let key = load_private_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/common/certs/sample.rsa"
        ))
        .unwrap();

        // Configure the server for TLS using the loaded certificate and key.
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth() // No client authentication required.
            .with_single_cert(cert, key) // Use a single certificate for the server.
            .map_err(|e| error(e.to_string()))
            .unwrap();

        // Wrap the server configuration in an Arc to enable shared ownership for async tasks.
        let server = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        // Accept a client connection and upgrade it to a TLS stream.
        let (server_stream, _) = listener.accept().await.unwrap();
        let tls_stream = server.accept(server_stream).await.unwrap();

        // Wrap the TLS stream in an abstraction compatible with Tokio's runtime.
        let io = TokioIo::new(tls_stream);

        // Serve incoming requests using Hyper's connection builder.
        hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
            .serve_connection(io, service_fn(echo)) // Use the `echo` handler for requests.
            .await
            .unwrap();
    });

    // Wait for all tasks to complete. Handles both client and server termination.
    while let Some(res) = tasks.join_next().await {
        // Ensure tasks do not fail unexpectedly.
        res.unwrap();
    }

    Ok(())
}
