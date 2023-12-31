use std::sync::Arc;

use tracing::{debug, error};

use crate::{
    util::http::{serve_http, serve_https},
    AppError, Config,
};

pub struct Server {
    pub router: axum_login::axum::Router,
    pub config: Arc<Config>,
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

            let https_router = self.router.clone();
            let ssl_config = self.config.website.bind_ssl_config.clone();
            tokio::spawn(async move {
                if let Err(e) = serve_https(https_addr, https_router, ssl_config).await {
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
