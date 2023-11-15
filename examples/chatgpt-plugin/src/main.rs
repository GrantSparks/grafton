use std::{net::SocketAddr, path::PathBuf};

use axum_server::tls_rustls::RustlsConfig;
use grafton_server::{
    server::create_grafton_router,
    server::start_http_server,
    server::start_https_server,
    tracing::{error, info},
    AppError, Config,
};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Load the configuration
    let config = match Config::load("./config") {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(AppError::ConfigError(e.to_string()));
        }
    };

    // Get the router
    let router = create_grafton_router(config.clone()).await.map_err(|e| {
        error!("Failed to create router: {}", e);
        e
    })?;

    let make_web_service = router.into_make_service();

    let http_addr = SocketAddr::from((config.website.bind_address, config.website.bind_ports.http));

    if config.website.bind_ssl_config.enabled {
        let https_addr =
            SocketAddr::from((config.website.bind_address, config.website.bind_ports.https));
        let ssl_config = RustlsConfig::from_pem_file(
            PathBuf::from(&config.website.bind_ssl_config.cert_path),
            PathBuf::from(&config.website.bind_ssl_config.key_path),
        )
        .await?;

        start_https_server(https_addr, make_web_service, ssl_config).await?;
    } else {
        start_http_server(http_addr, make_web_service).await?;
    };

    info!("Application started successfully");
    signal::ctrl_c().await?;
    info!("Server shut down gracefully");

    Ok(())
}
