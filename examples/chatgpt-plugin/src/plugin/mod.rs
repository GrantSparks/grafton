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

pub fn build_chatgpt_plugin_router(app_ctx: &Arc<AppContext>) -> AppRouter {
    build_specification_router(app_ctx).merge(build_manifest_router(app_ctx))
}
