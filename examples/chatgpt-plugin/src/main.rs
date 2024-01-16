mod plugin;

use {
    grafton_server::{tracing::info, AppError, Config, ServerBuilder, TracingLogger},
    tokio::signal,
};

use plugin::{build_chatgpt_plugin_router, build_todos_router};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let config = Config::load_from_dir("examples/chatgpt-plugin/config")?;

    let _logger_guard = TracingLogger::from_config(&config);

    let builder = ServerBuilder::new(config).await?;

    let server = builder
        .with_protected_router(build_todos_router)
        .with_unprotected_router(build_chatgpt_plugin_router)
        .build()
        .await?;

    server.start().await?;
    info!("Server started successfully");

    signal::ctrl_c().await?;
    info!("Server shutdown gracefully");

    Ok(())
}
