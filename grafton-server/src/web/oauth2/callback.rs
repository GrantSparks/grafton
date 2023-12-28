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
        AppError, AuthSession,
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
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        debug!("OAuth callback for provider: {}", provider);

        let old_state = session
            .get(CSRF_STATE_KEY)
            .await
            .map_err(|_| AppError::SessionStateError("Failed to retrieve CSRF state".to_string()))?
            .ok_or(AppError::MissingCSRFState)?;

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
                let provider_name = app_ctx
                    .config
                    .oauth_clients
                    .get(&provider)
                    .map(|client| client.display_name.clone())
                    .ok_or_else(|| AppError::ProviderNotFoundError(provider.clone()))?;

                return Ok((
                    StatusCode::UNAUTHORIZED,
                    LoginTemplate {
                        message: Some("Invalid CSRF state.".to_string()),
                        next: None,
                        provider_name,
                    },
                )
                    .into_response());
            }
            Err(e) => {
                error!("Internal error during authentication: {:?}", e);
                return Err(AppError::AuthenticationError(e.to_string()));
            }
        };

        if let Err(e) = auth_session.login(&user).await {
            error!("Error logging in the user: {:?}", e);
            return Err(AppError::LoginError(
                "Error logging in the user".to_string(),
            ));
        }

        match session.remove::<String>(NEXT_URL_KEY).await {
            Ok(Some(next)) if !next.is_empty() => Ok(Redirect::to(&next).into_response()),
            Ok(Some(_)) | Ok(None) => Ok(Redirect::to("/").into_response()),
            Err(e) => {
                error!("Session error: {:?}", e);
                Err(AppError::SessionError(
                    "Failed to retrieve next URL from session".to_string(),
                ))
            }
        }
    }
}
