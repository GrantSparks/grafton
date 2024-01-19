use std::sync::Arc;

use grafton_server::{
    axum::{routing::get, Router},
    model::Context,
    AxumRouter,
};

pub fn build_todos_router(app_ctx: &Arc<Context>) -> AxumRouter {
    let protected_home = &app_ctx.config.website.pages.with_root().protected_home;

    Router::new().route(protected_home, get(self::get::plugin_handler))
}

mod get {

    use super::Arc;

    use grafton_server::{
        axum::{
            extract::State,
            response::{IntoResponse, Json},
        },
        model::Context,
    };

    pub async fn plugin_handler(State(_app_ctx): State<Arc<Context>>) -> impl IntoResponse {
        let todos = vec![
            String::from("Collect underpants"),
            String::from("..."),
            String::from("Profit!"),
        ];
        Json(todos).into_response()
    }
}
