mod error;
mod model;
mod server;
mod util;
mod web;

#[cfg(feature = "rbac")]
mod rbac;

use error::AppError;
use server::start;
use tracing::info;
use util::Config;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    match Config::load("./config") {
        Ok(config) => match start(config).await {
            Ok(_) => info!("Application started successfully"),
            Err(e) => eprintln!("Failed to start application: {}", e),
        },
        Err(e) => eprintln!("Failed to load config: {}", e),
    }

    Ok(())
}
