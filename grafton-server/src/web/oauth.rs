use std::sync::Arc;

use axum_login::{
    axum,
    axum::{
        extract::{Path, Query},
        response::{IntoResponse, Redirect},
        routing::get,
    },
    http::StatusCode,
    tower_sessions::Session,
};
use oauth2::CsrfToken;
use serde::Deserialize;

use crate::{
    model::AppContext,
    web::auth::Credentials,
    web::auth::{LoginTemplate, NEXT_URL_KEY},
};

pub const CSRF_STATE_KEY: &str = "oauth.csrf-state";

#[derive(Debug, Clone, Deserialize)]
pub struct AuthzResp {
    code: String,
    state: CsrfToken,
}

pub fn router() -> axum::Router<Arc<AppContext>> {
    axum::Router::new().route("/oauth/:provider/callback", get(self::get::callback))
}

mod get {
    use crate::web::auth::AuthSession;

    use super::*;

    pub async fn callback(
        mut auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        Query(AuthzResp {
            code,
            state: new_state,
        }): Query<AuthzResp>,
    ) -> impl IntoResponse {
        let Ok(Some(old_state)) = session.get(CSRF_STATE_KEY) else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let creds = Credentials {
            code,
            old_state,
            new_state,
            provider
        };

        let user = match auth_session.authenticate(creds).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    LoginTemplate {
                        message: Some("Invalid CSRF state.".to_string()),
                        next: None,
                    },
                )
                    .into_response()
            }
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        if auth_session.login(&user).await.is_err() {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        if let Ok(Some(next)) = session.remove::<String>(NEXT_URL_KEY) {
            Redirect::to(&next).into_response()
        } else {
            Redirect::to("/").into_response()
        }
    }
}
