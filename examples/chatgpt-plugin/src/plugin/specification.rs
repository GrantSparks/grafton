use grafton_server::axum::{routing::get, Router};

use crate::AppRouter;

pub fn build_specification_router(openapi_yaml: &str) -> AppRouter {
    Router::new().route(openapi_yaml, get(self::get::openapi_handler))
}

mod get {

    use crate::plugin::config::ChatGptPlugin;

    use {axum_yaml::Yaml, grafton_server::axum::extract::State, openapiv3::OpenAPI};

    pub async fn openapi_handler(State(chatgpt_plugin): State<ChatGptPlugin>) -> Yaml<OpenAPI> {
        Yaml(chatgpt_plugin.openapi.clone())
    }
}
