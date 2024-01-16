use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::Arc,
};

use {
    askama_axum::IntoResponse,
    hyper::body::Incoming,
    hyper_util::{
        rt::{TokioExecutor, TokioIo},
        server::conn::auto::Builder as AutoBuilder,
    },
    pki_types::{CertificateDer, PrivateKeyDer},
    rustls_pemfile::{certs, pkcs8_private_keys},
    tokio::net::TcpListener,
    tokio_rustls::{rustls::ServerConfig, TlsAcceptor},
    tower::ServiceExt,
};

use crate::{
    axum::{extract::Request, Router},
    tracing::{debug, error},
    util::SslConfig,
    AppError,
};

fn create_tls_config(ssl_config: &SslConfig) -> Result<ServerConfig, AppError> {
    debug!("Creating TLS Config with SSL Config: {:?}", ssl_config);

    let certs = load_certs(Path::new(&ssl_config.cert_path))?;
    let key = load_keys(Path::new(&ssl_config.key_path))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    Ok(config)
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    debug!("Loading certificates from {:?}", path);

    if !path.exists() {
        error!("Certificate file path does not exist: {:?}", path);
        return Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
    }

    let file = match File::open(path) {
        Ok(file) => {
            debug!("Certificate file opened successfully");
            file
        }
        Err(e) => {
            error!("Failed to open certificate file: {:?}", e);
            return Err(e);
        }
    };

    let mut reader = BufReader::new(file);
    let mut cert_vec = Vec::new();

    for cert in certs(&mut reader) {
        match cert {
            Ok(cert) => {
                debug!("Certificate processed successfully");
                cert_vec.push(cert);
            }
            Err(_) => {
                error!("Invalid certificate encountered");
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"));
            }
        }
    }

    if cert_vec.is_empty() {
        error!("No certificates were loaded from the file");
        return Err(io::Error::new(io::ErrorKind::InvalidData, "no certs found"));
    }

    debug!("Certificates loaded successfully");
    Ok(cert_vec)
}

fn load_keys(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    debug!("Attempting to load keys from {:?}", path);

    if !path.exists() {
        error!("Path does not exist: {:?}", path);
        return Err(io::Error::new(io::ErrorKind::NotFound, "Path not found"));
    }

    let file = match File::open(path) {
        Ok(file) => {
            debug!("File opened successfully");
            file
        }
        Err(e) => {
            error!("Failed to open file: {:?}", e);
            return Err(e);
        }
    };

    let mut reader = BufReader::new(file);
    let keys: Result<Vec<_>, _> = pkcs8_private_keys(&mut reader).collect();

    match keys {
        Ok(keys) if keys.is_empty() => {
            error!("No keys found in the file");
            Err(io::Error::new(io::ErrorKind::NotFound, "no keys found"))
        }
        Ok(mut keys) => {
            debug!("Keys loaded successfully");
            Ok(PrivateKeyDer::from(keys.remove(0)))
        }
        Err(e) => {
            error!("Error reading keys: {:?}", e);
            Err(e)
        }
    }
}

pub async fn serve_https(
    addr: SocketAddr,
    router: Router,
    ssl_config: SslConfig,
) -> Result<(), AppError> {
    let server_config = create_tls_config(&ssl_config)?;
    let acceptor = TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind(&addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();
        let router_clone = router.clone();

        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    let io = TokioIo::new(tls_stream);

                    let service = hyper::service::service_fn(move |req: Request<Incoming>| {
                        let router = router_clone.clone();
                        async move {
                            match router.oneshot(req).await {
                                Ok(response) => Ok::<_, hyper::Error>(response),
                                Err(e) => {
                                    error!("Encountered an error: {:?}", e);
                                    Ok::<_, hyper::Error>(e.into_response())
                                }
                            }
                        }
                    });

                    if let Err(err) = AutoBuilder::new(TokioExecutor::new())
                        .serve_connection(io, service)
                        .await
                    {
                        error!("Error serving TLS connection: {:?}", err);
                    }
                }
                Err(e) => {
                    error!("Failed to accept a TLS connection: {:?}", e);
                }
            }
        });
    }
}

pub async fn serve_http(addr: SocketAddr, router: Router) -> Result<(), AppError> {
    debug!("Starting HTTP server at address {}", addr);

    let listener = TcpListener::bind(addr).await?;

    loop {
        let router_clone = router.clone();

        match listener.accept().await {
            Ok((stream, _)) => {
                let io = TokioIo::new(stream);

                tokio::task::spawn(async move {
                    let service = hyper::service::service_fn(move |req: Request<Incoming>| {
                        let router = router_clone.clone();
                        async move {
                            match router.oneshot(req).await {
                                Ok(response) => Ok::<_, hyper::Error>(response),
                                Err(e) => {
                                    error!("Encountered an error: {:?}", e);
                                    Ok::<_, hyper::Error>(e.into_response())
                                }
                            }
                        }
                    });

                    if let Err(err) = AutoBuilder::new(TokioExecutor::new())
                        .serve_connection(io, service)
                        .await
                    {
                        error!("Error serving connection: {:?}", err);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {:?}", e);
            }
        }
    }
}
