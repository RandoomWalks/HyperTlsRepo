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

// use std::{convert::Infallible, error::Error};

// use bytes::Bytes;
// use http::{header::CONTENT_TYPE, Request, Response};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::task::JoinSet;

// Also need this async function that's referenced in the test but not shown:
async fn echo(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, std::io::Error> {
    Ok(Response::new(Full::new(Bytes::new())))
}
// async fn echo(_req: Request<Body>) -> Result<Response<Body>, std::io::Error> {
//     Ok(Response::new(Body::from("Hello, World!")))
// }

fn error(err: String) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

fn load_certs(filename: &str) -> std::io::Result<Vec<CertificateDer<'static>>> {
    // Open certificate file.
    let certfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(certfile);

    // Load and return certificate.
    rustls_pemfile::certs(&mut reader).collect()
}

// Load private key from file.
fn load_private_key(filename: &str) -> std::io::Result<PrivateKeyDer<'static>> {
    // Open keyfile.
    let keyfile = std::fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = std::io::BufReader::new(keyfile);

    // Load and return a single private key.
    rustls_pemfile::private_key(&mut reader).map(|key| key.unwrap())
}

#[tokio::test]
async fn rustls_test() -> Result<(), Box<dyn Error + Send + Sync>> {
    let localhost = "127.0.0.1".to_owned();
    let listener = TcpListener::bind(format!("{}:0", localhost)).await?;
    let addr = listener.local_addr()?;

    let mut tasks = tokio::task::JoinSet::new();

    tasks.spawn(async move {
        let cert = load_certs(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/common/certs/sample.pem"
        ))
        .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add_parsable_certificates(cert);
        let tls = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(tls)
            .https_or_http()
            .enable_http1()
            .build();

        let client: Client<_, Empty<Bytes>> = Client::builder(TokioExecutor::new()).build(https);
        let uri = Uri::from_str(format!("https://localhost:{}", addr.port()).as_str()).unwrap();
        let response = client.get(uri).await.unwrap();
        assert_eq!(response.status(), 200);
        println!("client connected!");
    });

    tasks.spawn(async move {
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
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)
            .map_err(|e| error(e.to_string()))
            .unwrap();
        let server = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

        let (server_stream, _) = listener.accept().await.unwrap();
        let tls_stream = server.accept(server_stream).await.unwrap();
        let io = TokioIo::new(tls_stream);

        hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
            .serve_connection(io, service_fn(echo))
            .await
            .unwrap();

        // hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
        //     .serve_connection(io, hyper::service::service_fn(echo))
        //     .await
        //     .unwrap();
    });

    while let Some(res) = tasks.join_next().await {
        res.unwrap();
    }

    Ok(())
}
