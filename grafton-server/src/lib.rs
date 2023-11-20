mod error;
pub mod model;
pub mod server;
mod util;
mod web;

#[cfg(feature = "rbac")]
mod rbac;

pub use error::AppError;
pub use tracing;
pub use util::{read_config_from_dir, Config};
