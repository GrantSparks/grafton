use std::sync::Arc;

use axum_login::tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer};
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

pub async fn start(config: Config) -> Result<(), AppError> {
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
            // Convert 'e' to an appropriate variant of `AppError`
            // This might involve checking the type of 'e' and then deciding
            // which `AppError` variant to use
            return Err(AppError::from(e)); // Or use a specific `AppError` variant
        }
    };

    let auth_router = app.create_auth_router().await;

    Ok(())
}
