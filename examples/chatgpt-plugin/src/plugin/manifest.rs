use grafton_server::axum::{routing::get, Router};

use crate::AppRouter;

pub fn build_manifest_router(plugin_json: &str) -> AppRouter {
    Router::new().route(plugin_json, get(self::get::well_known_handler))
}

mod get {

    use grafton_server::axum::{
        extract::State,
        response::{Json, Redirect},
    };

    use crate::plugin::config::{ChatGptPlugin, Info};

    pub async fn well_known_handler(
        State(chatgpt_plugin): State<ChatGptPlugin>,
    ) -> Result<Json<Info>, Redirect> {
        Ok(Json(chatgpt_plugin.plugin_info.clone()))
    }
}
