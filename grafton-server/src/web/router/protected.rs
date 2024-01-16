use std::sync::Arc;

use askama::Template;

use crate::{
    axum::{routing::get, Router},
    core::AxumRouter,
    model::AppContext,
};

#[derive(Template)]
#[template(path = "protected.html")]
struct ProtectedTemplate<'a> {
    username: &'a str,
}

pub fn router(protected_home: String) -> AxumRouter {
    Router::new().route(&protected_home, get(self::get::protected))
}

mod get {

    use crate::{
        axum::{extract::State, http::StatusCode, response::IntoResponse},
        AuthSession,
    };

    use super::*;

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
}
