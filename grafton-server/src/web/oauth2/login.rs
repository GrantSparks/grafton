use {
    askama::Template,
    axum_login::axum::routing::{get, post},
    serde::Deserialize,
};

use crate::r#type::AxumRouter;

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
}

pub fn router() -> AxumRouter {
    AxumRouter::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
}

mod post {
    use {
        askama_axum::IntoResponse,
        axum_login::{
            axum::{extract::Path, http::StatusCode, response::Redirect, Form},
            tower_sessions::Session,
        },
    };

    use crate::web::{oauth2::AuthSession, oauth2::CSRF_STATE_KEY};

    use super::{NextUrl, NEXT_URL_KEY};

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

    use axum_login::axum::extract::Query;

    use super::{LoginTemplate, NextUrl};

    pub async fn login(Query(NextUrl { next }): Query<NextUrl>) -> LoginTemplate {
        LoginTemplate {
            message: None,
            next,
        }
    }
}
