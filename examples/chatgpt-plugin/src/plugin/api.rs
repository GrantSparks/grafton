use std::sync::Arc;

use grafton_server::{
    axum::{routing::get, Router},
    GraftonConfigProvider as _,
};

use crate::{AppContext, AppRouter};

pub fn build_todos_router(app_ctx: &Arc<AppContext>) -> AppRouter {
    let protected_home = &app_ctx
        .config
        .get_grafton_config()
        .website
        .pages
        .with_root()
        .protected_home;

    Router::new().route(protected_home, get(self::get::plugin_handler))
}

mod get {

    use super::{AppContext, Arc};

    use grafton_server::axum::{
        extract::State,
        response::{IntoResponse, Json},
    };

    pub async fn plugin_handler(State(_app_ctx): State<Arc<AppContext>>) -> impl IntoResponse {
        let todos = vec![
            String::from("Collect underpants"),
            String::from("..."),
            String::from("Profit!"),
        ];
        Json(todos).into_response()
    }
}
