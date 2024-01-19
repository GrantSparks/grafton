use std::sync::Arc;

use grafton_server::{model::Context, AxumRouter};

mod api;
pub use api::build_todos_router;

mod manifest;
use manifest::build_manifest_router;

mod specification;
use specification::build_specification_router;

pub fn build_chatgpt_plugin_router(app_ctx: &Arc<Context>) -> AxumRouter {
    build_specification_router(app_ctx).merge(build_manifest_router(app_ctx))
}
