use std::sync::Arc;

use askama::Template;
use axum::{debug_handler, extract::State, response::IntoResponse, routing::get, Router};
use http::StatusCode;

use crate::{model::AppContext, web::auth::AuthSession};

#[derive(Template)]
#[template(path = "protected.html")]
struct ProtectedTemplate<'a> {
    username: &'a str,
}

pub fn router() -> axum::Router<Arc<AppContext>> {
    Router::new().route("/", get(protected))
}

#[debug_handler]
pub async fn protected(
    State(_app_ctx): State<Arc<AppContext>>,
    auth_session: AuthSession,
) -> impl IntoResponse {
    match auth_session.user {
        Some(user) => ProtectedTemplate {
            username: &user.username,
        }
        .into_response(),

        None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}