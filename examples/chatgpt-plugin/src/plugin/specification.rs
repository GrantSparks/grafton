use std::sync::Arc;

use grafton_server::axum::{routing::get, Router};

use crate::{AppContext, AppRouter};

pub fn build_specification_router(openapi_yaml: &str) -> AppRouter {
    Router::new().route(openapi_yaml, get(self::get::openapi_handler))
}

mod get {

    use super::{AppContext, Arc};

    use {axum_yaml::Yaml, grafton_server::axum::extract::State, openapiv3::OpenAPI};

    pub async fn openapi_handler(State(app_ctx): State<Arc<AppContext>>) -> Yaml<OpenAPI> {
        Yaml(app_ctx.config.chatgpt_plugin.openapi.clone())
    }
}
