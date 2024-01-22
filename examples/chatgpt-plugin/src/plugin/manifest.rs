use std::sync::Arc;

use grafton_server::{
    axum::{routing::get, Router},
    GraftonConfigProvider,
};

use crate::{AppContext, AppRouter};

pub fn build_manifest_router(app_ctx: &Arc<AppContext>) -> AppRouter {
    let plugin_json = app_ctx
        .config
        .get_grafton_config()
        .website
        .pages
        .with_root()
        .plugin_json;

    Router::new().route(&plugin_json, get(self::get::well_known_handler))
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
        Ok(Json(app_ctx.config.plugin_info.clone()))
    }
}
