use std::sync::Arc;

use {
    askama::Template,
    axum_login::axum::{routing::get, Router},
};

use crate::model::{AppContext, AxumRouter};

#[derive(Template)]
#[template(path = "protected.html")]
struct ProtectedTemplate<'a> {
    username: &'a str,
}

pub fn router() -> AxumRouter {
    Router::new().route("/", get(self::get::protected))
}

mod get {

    use super::*;

    use axum_login::axum::{
        debug_handler, extract::State, http::StatusCode, response::IntoResponse,
    };

    use crate::web::oauth2::AuthSession;

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
}
