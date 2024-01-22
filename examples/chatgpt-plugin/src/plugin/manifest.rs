use std::sync::Arc;

use grafton_server::axum::{routing::get, Router};

use crate::{AppContext, AppRouter};

pub fn build_manifest_router(plugin_json: &str) -> AppRouter {
    Router::new().route(plugin_json, get(self::get::well_known_handler))
}

mod get {

    use grafton_server::axum::{
        extract::State,
        response::{Json, Redirect},
    };

    use crate::plugin::config::Info;

    use super::{AppContext, Arc};

    pub async fn well_known_handler(
        State(app_ctx): State<Arc<AppContext>>,
    ) -> Result<Json<Info>, Redirect> {
        Ok(Json(app_ctx.config.chatgpt_plugin.plugin_info.clone()))
    }
}
