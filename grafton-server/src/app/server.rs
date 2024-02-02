use std::{net::IpAddr, str::FromStr as _, sync::Arc};

use crate::{
    axum::Router,
    tracing::{debug, error},
    util::http::{serve_http, serve_https},
    ServerConfigProvider, SslConfig,
};

pub struct Server<C>
where
    C: ServerConfigProvider,
{
    pub router: Router,
    pub config: Arc<C>,
}

impl<C> Server<C>
where
    C: ServerConfigProvider,
{
    pub fn start(self) {
        let server_config = self.config.get_server_config();
        let bind_address_str = server_config.website.bind_address.to_string();
        let ports = server_config.website.bind_ports.clone();
        let ssl_config = server_config.website.bind_ssl_config.clone();

        if ssl_config.enabled {
            self.start_https_server(&bind_address_str, ports.https, ssl_config);
        } else {
            self.start_http_server(&bind_address_str, ports.http);
        }

        debug!("Server startup initiated");
    }

    fn start_https_server(&self, addr: &str, port: u16, ssl_config: SslConfig) {
        let https_addr = match IpAddr::from_str(addr) {
            Ok(ip) => (ip, port).into(),
            Err(e) => {
                error!("Invalid IP address: {}", e);
                return;
            }
        };

        let https_router = self.router.clone();

        tokio::spawn(async move {
            if let Err(e) = serve_https(https_addr, https_router, ssl_config).await {
                error!("Failed to start HTTPS server: {}", e);
            }
        });
    }

    fn start_http_server(&self, addr: &str, port: u16) {
        let http_addr = match IpAddr::from_str(addr) {
            Ok(ip) => (ip, port).into(),
            Err(e) => {
                error!("Invalid IP address: {}", e);
                return;
            }
        };

        let http_router = self.router.clone();

        tokio::spawn(async move {
            if let Err(e) = serve_http(http_addr, http_router).await {
                error!("Failed to start HTTP server: {}", e);
            }
        });
    }
}
