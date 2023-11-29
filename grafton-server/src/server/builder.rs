use std::{fs, io::BufReader, net::SocketAddr, sync::Arc};

use askama_axum::IntoResponse;
use axum_login::{
    axum::{extract::Request, Router},
    tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer},
};
use hyper::body::Incoming;
use hyper_util::{
    rt::{TokioExecutor, TokioIo},
    server::conn::auto::Builder as AutoBuilder,
};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile as pemfile;
use time::Duration;
use tls_listener::TlsListener;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower::ServiceExt;
use tracing::{error, warn};

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
        let http_addr = (
            self.config.website.bind_address,
            self.config.website.bind_ports.http,
        )
            .into();

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
            tokio::spawn(async move {
                if let Err(e) = serve_http(http_addr, self.router.clone()).await {
                    error!("Failed to start HTTP server: {}", e);
                }
            });
        }

        Ok(())
    }
}

fn create_tls_acceptor(ssl_config: &SslConfig) -> Result<TlsAcceptor, AppError> {
    let cert_file = fs::File::open(&ssl_config.cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_chain = pemfile::certs(&mut cert_reader)?
        .into_iter()
        .map(Certificate)
        .collect::<Vec<_>>();

    let key_file = fs::File::open(&ssl_config.key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let keys = pemfile::pkcs8_private_keys(&mut key_reader)?
        .into_iter()
        .map(PrivateKey)
        .collect::<Vec<_>>();

    if keys.is_empty() {
        return Err(AppError::ConfigError(
            "No private keys found in key file".to_string(),
        ));
    }

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth() // GMS: TODO: verify correct setting for axum-login
        .with_single_cert(cert_chain, keys[0].clone())?;

    let acceptor = TlsAcceptor::from(Arc::new(config));
    Ok(acceptor)
}

pub struct ServerBuilder {
    app_ctx: Arc<AppContext>,
    inner_router: axum_login::axum::Router<std::sync::Arc<AppContext>>,
}

impl ServerBuilder {
    pub async fn new(config: Config) -> Result<Self, AppError> {
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

        Ok(Self {
            app_ctx: context,
            inner_router,
        })
    }

    pub fn build(self) -> Result<Server, AppError> {
        let config = self.app_ctx.config.clone();
        let router = self.inner_router.with_state(self.app_ctx);

        Ok(Server { router, config })
    }
}

fn create_session_layer() -> SessionManagerLayer<MemoryStore> {
    let session_store = MemoryStore::default();
    SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)))
}

async fn serve_https(
    addr: SocketAddr,
    router: Router,
    tls_acceptor: TlsAcceptor,
) -> Result<(), AppError> {
    let tcp_listener = TcpListener::bind(addr).await.map_err(AppError::IoError)?;

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
                        error!("Error serving connection: {:?}", err);
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

    Ok(())
}

async fn serve_http(addr: SocketAddr, router: Router) -> Result<(), AppError> {
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
