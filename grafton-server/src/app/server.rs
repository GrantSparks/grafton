use std::sync::Arc;

use crate::{
    axum::Router,
    tracing::{debug, error},
    util::http::{serve_http, serve_https},
    GraftonConfigProvider,
};

pub struct Server<C>
where
    C: GraftonConfigProvider,
{
    pub router: Router,
    pub config: Arc<C>,
}

impl<C> Server<C>
where
    C: GraftonConfigProvider,
{
    pub fn start(self) {
        debug!("Starting server with configuration: {:?}", self.config);

        if self
            .config
            .get_grafton_config()
            .website
            .bind_ssl_config
            .enabled
        {
            let https_addr = (
                self.config.get_grafton_config().website.bind_address,
                self.config.get_grafton_config().website.bind_ports.https,
            )
                .into();

            let https_router = self.router.clone();
            let ssl_config = self
                .config
                .get_grafton_config()
                .website
                .bind_ssl_config
                .clone();
            tokio::spawn(async move {
                if let Err(e) = serve_https(https_addr, https_router, ssl_config).await {
                    error!("Failed to start HTTPS server: {}", e);
                }
            });
        } else {
            let http_addr = (
                self.config.get_grafton_config().website.bind_address,
                self.config.get_grafton_config().website.bind_ports.http,
            )
                .into();

            let http_router = self.router;
            tokio::spawn(async move {
                if let Err(e) = serve_http(http_addr, http_router).await {
                    error!("Failed to start HTTP server: {}", e);
                }
            });
        }

        debug!("Server started successfully");
    }
}
