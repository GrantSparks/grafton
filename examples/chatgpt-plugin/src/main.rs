mod plugin;

use {
    grafton_server::{tracing::info, Builder, Config, Error, Logger},
    tokio::signal,
};

use plugin::{build_chatgpt_plugin_router, build_todos_router};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::load_from_dir("examples/chatgpt-plugin/config")?;

    let _logger_guard = Logger::from_config(&config);

    let builder = Builder::new(config)?;

    let server = builder
        .with_protected_router(build_todos_router)
        .with_unprotected_router(build_chatgpt_plugin_router)
        .build()
        .await?;

    server.start();
    info!("Server started successfully");

    signal::ctrl_c().await?;
    info!("Server shutdown gracefully");

    Ok(())
}
