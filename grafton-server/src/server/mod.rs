use std::{error::Error, net::SocketAddr, sync::Arc};

use axum::routing::IntoMakeService;
use axum_login::{
    axum,
    tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer},
};
use axum_server::{tls_rustls::RustlsConfig, Server};
use time::Duration;
use tracing::info;

use crate::{
    error::AppError,
    model::AppContext,
    util::{Config, TracingLogger},
    web::App,
};

fn create_session_layer() -> SessionManagerLayer<MemoryStore> {
    let session_store = MemoryStore::default();
    SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)))
}

pub async fn create_grafton_router(config: Config) -> Result<axum::Router, AppError> {
    let _logger_guard = TracingLogger::from_config(&config);

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

    let app_ctx = Arc::new(context);
    info!("Application started with context: {:?}", app_ctx);

    let session_layer = create_session_layer();

    let app_result = App::new(app_ctx.clone(), session_layer).await;
    let app = match app_result {
        Ok(app) => app,
        Err(e) => {
            return Err(AppError::from(e));
        }
    };

    let auth_router = app.create_auth_router().await;

    let router_with_state = auth_router.with_state(app_ctx.clone());

    Ok(router_with_state)
}

pub async fn start_https_server(
    https_addr: SocketAddr,
    web_service: IntoMakeService<axum::Router>,
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

pub async fn start_http_server(
    http_addr: SocketAddr,
    web_service: IntoMakeService<axum::Router>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    info!("http server listening on {}", http_addr);

    tokio::task::spawn(async move { Server::bind(http_addr).serve(web_service).await });

    Ok(())
}
