use rustls::{Certificate, PrivateKey, ServerConfig};
use std::fs::File;
use std::io::{self, BufReader};

/// Loads the TLS configuration with certificate and private key.
fn load_tls_config() -> io::Result<ServerConfig> {
    // Paths to your certificate and private key
    let cert_path = "certs/server.crt";
    let key_path = "certs/server.key";

    // Load the certificate and private key
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("TLS config error: {}", e)))
}

/// Loads certificates from a PEM file.
fn load_certs(path: &str) -> io::Result<Vec<Certificate>> {
    let cert_file = File::open(path)?;
    let mut reader = BufReader::new(cert_file);
    rustls_pemfile::certs(&mut reader)
        .map(|certs| certs.into_iter().map(Certificate).collect())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid certificates"))
}

/// Loads a private key from a PEM file.
fn load_private_key(path: &str) -> io::Result<PrivateKey> {
    let key_file = File::open(path)?;
    let mut reader = BufReader::new(key_file);

    rustls_pemfile::read_all(&mut reader)
        .ok()
        .and_then(|items| {
            items
                .into_iter()
                .find_map(|item| match item {
                    rustls_pemfile::Item::RSAKey(key)
                    | rustls_pemfile::Item::PKCS8Key(key)
                    | rustls_pemfile::Item::ECKey(key) => Some(PrivateKey(key)),
                    _ => None,
                })
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"))
}
