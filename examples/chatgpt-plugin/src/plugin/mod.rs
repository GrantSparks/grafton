use std::sync::Arc;

use grafton_server::{model::AppContext, AxumRouter};

mod api;
pub use api::build_todos_router;

mod manifest;
use manifest::build_manifest_router;

mod specification;
use specification::build_specification_router;

pub fn build_chatgpt_plugin_router(app_ctx: Arc<AppContext>) -> AxumRouter {
    build_specification_router(app_ctx.clone()).merge(build_manifest_router(app_ctx.clone()))
}
