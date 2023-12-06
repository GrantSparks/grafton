use {
    askama::Template,
    askama_axum::IntoResponse,
    axum_login::axum::{
        extract::Path,
        http::StatusCode,
        routing::{get, post},
    },
    serde::Deserialize,
};

use crate::core::AxumRouter;

pub const NEXT_URL_KEY: &str = "auth.next-url";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub message: Option<String>,
    pub next: Option<String>,
    pub provider_name: String,
}

pub fn router() -> AxumRouter {
    AxumRouter::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
}

mod post {
    use axum_login::{
        axum::{response::Redirect, Form},
        tower_sessions::Session,
    };

    use crate::{web::oauth2::CSRF_STATE_KEY, AuthSession};

    use super::{IntoResponse, NextUrl, Path, StatusCode, NEXT_URL_KEY};

    /// Redirects to the OAuth2 provider's authorization URL.
    pub async fn login(
        auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> impl IntoResponse {
        match auth_session.backend.authorize_url(provider.clone()) {
            Ok((url, token)) => {
                session
                    .insert(CSRF_STATE_KEY, token.secret())
                    .expect("Serialization should not fail.");
                session
                    .insert(NEXT_URL_KEY, next)
                    .expect("Serialization should not fail.");

                Redirect::to(url.as_str()).into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

mod get {

    use std::sync::Arc;

    use axum::{debug_handler, extract::State};
    use axum_login::axum::extract::Query;

    use crate::model::AppContext;

    use super::{IntoResponse, LoginTemplate, NextUrl, Path, StatusCode};

    #[debug_handler]
    pub async fn login(
        Query(NextUrl { next }): Query<NextUrl>,
        Path(provider): Path<String>,
        State(app_ctx): State<Arc<AppContext>>,
    ) -> Result<LoginTemplate, impl IntoResponse> {
        match app_ctx.config.oauth_clients.get(&provider) {
            Some(client) => {
                let provider_name = &client.display_name;
                Ok(LoginTemplate {
                    message: None,
                    next,
                    provider_name: provider_name.clone(),
                })
            }
            None => Err(StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        }
    }
}
