use std::{error::Error, net::SocketAddr, path::PathBuf, sync::Arc};

use axum_login::{
    axum::routing::IntoMakeService,
    tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer},
};
use axum_server::tls_rustls::RustlsConfig;
use time::Duration;
use tracing::{error, info};

use crate::{model::AppContext, web::App, AppError, Config};

pub struct Server {
    router: axum_login::axum::Router,
    config: Arc<Config>,
}

impl Server {
    pub async fn start(self) -> Result<(), AppError> {
        let make_web_service = self.router.into_make_service();

        let http_addr = SocketAddr::from((
            self.config.website.bind_address,
            self.config.website.bind_ports.http,
        ));

        if self.config.website.bind_ssl_config.enabled {
            let https_addr = SocketAddr::from((
                self.config.website.bind_address,
                self.config.website.bind_ports.https,
            ));
            let ssl_config = RustlsConfig::from_pem_file(
                PathBuf::from(&self.config.website.bind_ssl_config.cert_path),
                PathBuf::from(&self.config.website.bind_ssl_config.key_path),
            )
            .await?;

            match start_https_server(https_addr, make_web_service, ssl_config).await {
                Ok(_) => info!("HTTPS server started successfully"),
                Err(e) => error!("Failed to start HTTPS server: {}", e),
            }
        } else {
            match start_http_server(http_addr, make_web_service).await {
                Ok(_) => info!("HTTP server started successfully"),
                Err(e) => error!("Failed to start HTTP server: {}", e),
            }
        };

        Ok(())
    }
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
                use crate::rbac::initialize;
                let oso = initialize(&config)?;
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
            Err(e) => {
                return Err(e);
            }
        };

        let inner_router = app.create_auth_router().await;

        Ok(Self {
            app_ctx: context,
            inner_router,
        })
    }

    pub async fn build(self) -> Result<Server, AppError> {
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

async fn start_https_server(
    https_addr: SocketAddr,
    web_service: IntoMakeService<axum_login::axum::Router>,
    config: RustlsConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("https server listening on {}", https_addr);

    tokio::task::spawn(async move {
        axum_server::bind_rustls(https_addr, config)
            .serve(web_service)
            .await
    });

    Ok(())
}

async fn start_http_server(
    http_addr: SocketAddr,
    web_service: IntoMakeService<axum_login::axum::Router>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("http server listening on {}", http_addr);

    tokio::task::spawn(async move {
        axum_server::Server::bind(http_addr)
            .serve(web_service)
            .await
    });

    Ok(())
}
