mod api;
use std::sync::Arc;

pub use api::build_todos_router;

mod manifest;
use manifest::build_manifest_router;

mod specification;
use specification::build_specification_router;

pub mod config;
pub use config::Config;

use crate::{AppContext, AppRouter};

pub fn build_chatgpt_plugin_router(app_ctx: &Arc<AppContext>) -> AppRouter {
    build_specification_router(&app_ctx.config.chatgpt_plugin.openapi_yaml).merge(
        build_manifest_router(&app_ctx.config.chatgpt_plugin.plugin_json),
    )
}
