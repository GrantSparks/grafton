mod config;
use std::sync::Arc;

pub use config::Config;

mod logger;
pub use logger::TracingLogger;

use crate::AppError;

mod token_expander;

pub fn read_config_from_dir(config_dir: &str) -> Result<Arc<Config>, AppError> {
    let config = Config::load(config_dir)?;
    Ok(Arc::new(config))
}
