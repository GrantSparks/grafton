use {
    axum_login::{
        axum::{
            extract::{Path, Query},
            http::StatusCode,
            response::{IntoResponse, Redirect},
            routing::get,
        },
        tower_sessions::Session,
    },
    tracing::{debug, error, warn},
};

use crate::core::AxumRouter;

pub fn router() -> AxumRouter {
    AxumRouter::new().route("/oauth/:provider/callback", get(self::get::callback))
}

mod get {
    use std::sync::Arc;

    use axum::extract::State;

    use crate::{
        model::AppContext,
        web::{
            oauth2::{
                login::{LoginTemplate, NEXT_URL_KEY},
                AuthzResp, CSRF_STATE_KEY,
            },
            Credentials,
        },
        AuthSession,
    };

    use super::*;

    pub async fn callback(
        mut auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        Query(AuthzResp {
            code,
            state: new_state,
        }): Query<AuthzResp>,
        State(app_ctx): State<Arc<AppContext>>,
    ) -> impl IntoResponse {
        debug!("OAuth callback for provider: {}", provider);

        let Ok(Some(old_state)) = session.get(CSRF_STATE_KEY) else {
            warn!("CSRF state missing or invalid");
            return StatusCode::BAD_REQUEST.into_response();
        };

        let creds = Credentials {
            code,
            old_state,
            new_state,
            provider: provider.clone(),
        };

        let user = match auth_session.authenticate(creds).await {
            Ok(Some(user)) => {
                debug!("User authenticated successfully");
                user
            }
            Ok(None) => {
                warn!("Invalid CSRF state, authentication failed");

                let provider_name = match app_ctx.config.oauth_clients.get(&provider) {
                    Some(client) => client.display_name.clone(),
                    None => {
                        error!("Provider not found: {}", provider);
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                };

                return (
                    StatusCode::UNAUTHORIZED,
                    LoginTemplate {
                        message: Some("Invalid CSRF state.".to_string()),
                        next: None,
                        provider_name,
                    },
                )
                    .into_response();
            }
            Err(e) => {
                error!("Internal error during authentication: {:?}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        if auth_session.login(&user).await.is_err() {
            error!("Error logging in the user");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }

        if let Ok(Some(next)) = session.remove::<String>(NEXT_URL_KEY) {
            debug!("Redirecting to next URL: {}", next);
            Redirect::to(&next).into_response()
        } else {
            debug!("Redirecting to home page");
            Redirect::to("/").into_response()
        }
    }
}
