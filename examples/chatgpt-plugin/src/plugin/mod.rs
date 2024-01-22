use std::sync::Arc;

mod api;
pub use api::build_todos_router;

mod manifest;
use manifest::build_manifest_router;

mod specification;
use specification::build_specification_router;

pub mod config;
pub use config::Info;

use crate::{AppContext, AppRouter};

use grafton_server::GraftonConfigProvider;

pub fn build_chatgpt_plugin_router(app_ctx: &Arc<AppContext>) -> AppRouter {
    let openapi_yaml = app_ctx
        .config
        .get_grafton_config()
        .website
        .pages
        .with_root()
        .openapi_yaml;

    let plugin_json = app_ctx
        .config
        .get_grafton_config()
        .website
        .pages
        .with_root()
        .plugin_json;

    build_specification_router(&openapi_yaml).merge(build_manifest_router(&plugin_json))
}
