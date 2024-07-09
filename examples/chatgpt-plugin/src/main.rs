mod plugin;
use grafton_auth::{Builder, Error};
use grafton_server::{tracing::info, Context, GraftonRouter, Logger};
use plugin::{build_chatgpt_plugin_router, build_todos_router, Config};

use {grafton_config::load_config_from_dir, tokio::signal};

type AppContext = Context<Config>;
type AppRouter = GraftonRouter<Config>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config: Config = load_config_from_dir("examples/chatgpt-plugin/config")?;

    let _logger_guard = Logger::from_config(&config.base.base);
    let builder_result = Builder::new(config);

    match builder_result {
        Ok(builder) => {
            let server_result = builder
                .with_protected_router(build_todos_router)
                .with_unprotected_router(build_chatgpt_plugin_router)
                .build()
                .await;

            match server_result {
                Ok(server) => {
                    server.start();
                    info!("Server started successfully");

                    signal::ctrl_c().await?;
                    info!("Server shutdown gracefully");

                    Ok(())
                }
                Err(e) => {
                    eprintln!("Failed to build the server: {}", e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create the builder: {}", e);
            Err(e)
        }
    }
}
