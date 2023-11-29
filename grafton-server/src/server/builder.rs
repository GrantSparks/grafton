use std::{fs, io::BufReader, net::SocketAddr, sync::Arc};

use {
    askama_axum::IntoResponse,
    axum_login::{
        axum::{extract::Request, Router},
        tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer},
    },
    hyper::body::Incoming,
    hyper_util::{
        rt::{TokioExecutor, TokioIo},
        server::conn::auto::Builder as AutoBuilder,
    },
    rustls::{Certificate, PrivateKey, ServerConfig},
    rustls_pemfile as pemfile,
    time::Duration,
    tls_listener::{rustls::TlsAcceptor, TlsListener},
    tokio::net::TcpListener,
    tower::ServiceExt,
    tracing::{debug, error, warn},
};

use crate::{
    model::AppContext,
    util::{Config, SslConfig},
    web::App,
    AppError,
};

pub struct Server {
    router: axum_login::axum::Router,
    config: Arc<Config>,
}

impl Server {
    pub async fn start(self) -> Result<(), AppError> {
        debug!("Starting server with configuration: {:?}", self.config);

        if self.config.website.bind_ssl_config.enabled {
            let https_addr = (
                self.config.website.bind_address,
                self.config.website.bind_ports.https,
            )
                .into();

            let tls_acceptor = create_tls_acceptor(&self.config.website.bind_ssl_config)?;

            let https_router = self.router.clone();
            tokio::spawn(async move {
                if let Err(e) = serve_https(https_addr, https_router, tls_acceptor).await {
                    error!("Failed to start HTTPS server: {}", e);
                }
            });
        } else {
            let http_addr = (
                self.config.website.bind_address,
                self.config.website.bind_ports.http,
            )
                .into();

            let http_router = self.router.clone();
            tokio::spawn(async move {
                if let Err(e) = serve_http(http_addr, http_router).await {
                    error!("Failed to start HTTP server: {}", e);
                }
            });
        }

        debug!("Server started successfully");
        Ok(())
    }
}

fn create_tls_acceptor(ssl_config: &SslConfig) -> Result<TlsAcceptor, AppError> {
    debug!("Creating TLS Acceptor with SSL Config: {:?}", ssl_config);

    let cert_file = match fs::File::open(&ssl_config.cert_path) {
        Ok(file) => file,
        Err(e) => {
            error!("Failed to open certificate file: {:?}", e);
            return Err(AppError::IoError(e));
        }
    };
    let mut cert_reader = BufReader::new(cert_file);

    let cert_chain = match pemfile::certs(&mut cert_reader) {
        Ok(certs) => certs.into_iter().map(Certificate).collect::<Vec<_>>(),
        Err(_) => {
            error!("Failed to read certificates from PEM file");
            return Err(AppError::InvalidCertificate);
        }
    };

    let key_file = fs::File::open(&ssl_config.key_path)?;
    let mut key_reader = BufReader::new(key_file);

    let ec_keys = rustls_pemfile::ec_private_keys(&mut key_reader).map_err(|e| {
        AppError::InvalidPrivateKey {
            file_path: ssl_config.key_path.clone(),
            error: e.to_string(),
        }
    })?;

    if ec_keys.is_empty() {
        error!(
            "No EC private keys found in key file: {}",
            ssl_config.key_path
        );
        return Err(AppError::NoPrivateKey {
            file_path: ssl_config.key_path.clone(),
            error: "The file does not contain valid SEC1 EC private keys or is empty.".to_string(),
        });
    }

    let private_key = PrivateKey(ec_keys[0].clone());

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth() // TODO: verify setting for axum-login
        .with_single_cert(cert_chain, private_key)?;

    let acceptor: TlsAcceptor = Arc::new(config).into();

    debug!("TLS Acceptor created successfully");
    Ok(acceptor)
}

pub struct ServerBuilder {
    app_ctx: Arc<AppContext>,
    inner_router: axum_login::axum::Router<std::sync::Arc<AppContext>>,
}

impl ServerBuilder {
    pub async fn new(config: Config) -> Result<Self, AppError> {
        debug!("Initializing ServerBuilder with config: {:?}", config);

        let context = {
            #[cfg(feature = "rbac")]
            {
                use crate::rbac;
                let oso = rbac::initialize(&config)?;
                AppContext::new(config, oso)?
            }
            #[cfg(not(feature = "rbac"))]
            {
                AppContext::new(config)?
            }
        };

        let context = Arc::new(context);

        let session_layer = create_session_layer();

        let app_result = App::new(context.clone(), session_layer).await;
        let app = match app_result {
            Ok(app) => app,
            Err(e) => return Err(e),
        };

        let inner_router = app.create_auth_router();

        debug!("ServerBuilder initialized");
        Ok(Self {
            app_ctx: context,
            inner_router,
        })
    }

    pub fn build(self) -> Result<Server, AppError> {
        debug!("Building server");

        let config = self.app_ctx.config.clone();
        let router = self.inner_router.with_state(self.app_ctx);

        debug!("Server built successfully");
        Ok(Server { router, config })
    }
}

fn create_session_layer() -> SessionManagerLayer<MemoryStore> {
    debug!("Creating session layer");

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)));

    debug!("Session layer created");
    session_layer
}

async fn serve_https(
    addr: SocketAddr,
    router: Router,
    tls_acceptor: TlsAcceptor,
) -> Result<(), AppError> {
    debug!("Starting HTTPS server at address {}", addr);

    let tcp_listener = TcpListener::bind(addr).await?;

    let mut listener = TlsListener::new(tls_acceptor, tcp_listener);

    loop {
        let router_clone = router.clone();

        match listener.accept().await {
            Some(Ok((stream, _))) => {
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
                        error!("Error serving connection: {:?}, Addr: {}", err, addr);
                    }
                });
            }
            Some(Err(err)) => {
                if let Some(remote_addr) = err.peer_addr() {
                    warn!("[client {remote_addr}] Error accepting connection: {}", err);
                } else {
                    warn!("Error accepting connection: {}", err);
                }
            }
            None => break, // Exit the loop if None is returned
        }
    }

    debug!("HTTPS server loop ended");
    Ok(())
}

async fn serve_http(addr: SocketAddr, router: Router) -> Result<(), AppError> {
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
