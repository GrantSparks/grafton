use grafton_server::{tracing::info, AppError, Config, ServerBuilder, TracingLogger};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Config::load_from_dir("examples/chatgpt-plugin/config")?;

    let _logger_guard = TracingLogger::from_config(&config);

    let builder = ServerBuilder::new(config).await?;

    let server = builder.build().await?;

    server.start().await?;
    info!("Server started successfully");

    signal::ctrl_c().await?;
    info!("Server shutdown gracefully");

    Ok(())
}
