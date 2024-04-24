mod plugin;
use plugin::{build_chatgpt_plugin_router, build_todos_router, Config};

use {
    grafton_config::load_config_from_dir,
    grafton_server::{model::Context, tracing::info, AxumRouter, Builder, Error, Logger},
    tokio::signal,
};

type AppContext = Context<Config>;
type AppRouter = AxumRouter<Config>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config: Config = load_config_from_dir("examples/chatgpt-plugin/config")?;

    let _logger_guard = Logger::from_config(&config.base);

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
