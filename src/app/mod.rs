use std::sync::Arc;

#[cfg(feature = "rbac")]
use crate::auth::oso::initialize_oso;

use crate::{
    error::AppError,
    model::AppContext,
    util::{Config, TracingLogger},
};

pub fn start(config: Config) -> Result<(), AppError> {
    let _logger_guard = TracingLogger::from_config(&config);

    #[cfg(feature = "rbac")]
    let oso = initialize_oso(&config)?;

    #[cfg(feature = "rbac")]
    let context = AppContext::new(config, oso)?;

    #[cfg(not(feature = "rbac"))]
    let context = AppContext::new(config)?;

    let app_ctx = Arc::new(context);

    println!("Application started with context: {:?}", app_ctx);

    Ok(())
}
