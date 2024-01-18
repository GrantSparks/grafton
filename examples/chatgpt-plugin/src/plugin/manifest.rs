use std::sync::Arc;

use grafton_server::{
    axum::{routing::get, Router},
    model::AppContext,
    AxumRouter,
};

pub fn build_manifest_router(app_ctx: Arc<AppContext>) -> AxumRouter {
    let plugin_json = &app_ctx.config.website.pages.with_root().plugin_json;

    Router::new().route(plugin_json, get(self::get::well_known_handler))
}

mod get {

    use super::*;

    use grafton_server::{
        axum::{
            extract::State,
            response::{Json, Redirect},
        },
        model::AppContext,
        PluginInfo,
    };

    pub async fn well_known_handler(
        State(app_ctx): State<Arc<AppContext>>,
    ) -> Result<Json<PluginInfo>, Redirect> {
        Ok(Json(app_ctx.config.plugin_info.clone()))
    }
}
