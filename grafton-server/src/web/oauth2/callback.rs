use axum_login::tower_sessions::Session;

use tracing::{debug, error, info, warn};

use crate::{
    axum::{
        extract::{Path, Query},
        response::{IntoResponse, Redirect},
        routing::{get, post},
    },
    core::AxumRouter,
    ServerConfigProvider,
};

pub fn router<C>() -> AxumRouter<C>
where
    C: ServerConfigProvider,
{
    AxumRouter::new()
        .route("/oauth/:provider/callback", get(self::get::callback))
        .route("/oauth/token", post(self::post::get_access_token))
}

mod post {
    use crate::{
        axum::{extract::State, Form, Json},
        web::{oauth2::login::OpenAiAuthParams, Credentials},
        AuthSession, Config, Error,
    };
    use askama_axum::IntoResponse;

    use super::{debug, error, info, warn};
    use serde_json::json;

    pub async fn get_access_token(
        mut auth_session: AuthSession,
        State(config): State<Config>,
        Form(OpenAiAuthParams {
            grant_type,
            client_id,
            client_secret,
            code,
            redirect_uri,
        }): Form<OpenAiAuthParams>,
    ) -> Result<impl IntoResponse, Error> {
        info!("{}", format!(
            "Received access token request with the following parameters: client_id={}, client_secret={}, grant_type={}, code={:?}, redirect_uri={}",
            client_id, client_secret, grant_type, code, redirect_uri
        ));

        let provider = "github".to_string();

        if let Some(oauth_client) = config.oauth_providers.get(&provider) {
            if let Some(userinfo_uri) = oauth_client.extra.get("userinfo_uri") {
                let userinfo_uri = userinfo_uri.as_str().unwrap().to_string();
                let creds = Credentials {
                    code: code.clone(),
                    provider,
                    userinfo_uri,
                };

                let user = match auth_session.authenticate(creds).await {
                    Ok(Some(user)) => {
                        debug!("User authenticated successfully");
                        user
                    }
                    Ok(None) => {
                        warn!("Authentication succeeded but no user was found");
                        return Err(Error::AuthenticationError("User not found".to_string()));
                    }
                    Err(e) => {
                        error!("Internal error during authentication: {:?}", e);
                        return Err(Error::AuthenticationError(e.to_string()));
                    }
                };

                if let Err(e) = auth_session.login(&user).await {
                    error!("Error logging in the user: {:?}", e);
                    return Err(Error::LoginError("Error logging in the user".to_string()));
                }

                let response_body = json!({
                    "access_token": "example_token",
                    "token_type": "bearer",
                    "refresh_token": "example_token",
                    "expires_in": 59,
                });

                Ok(Json(response_body))
            } else {
                return Err(Error::ClientConfigNotFound("userinfo_uri".to_string()));
            }
        } else {
            return Err(Error::ProviderNotFoundError(provider));
        }
    }
}

mod get {
    use url::Url;

    use crate::{
        axum::extract::State,
        web::oauth2::{login::NEXT_URL_KEY, AuthzResp, CSRF_STATE_KEY},
        Config, Error,
    };

    use super::{debug, error, IntoResponse, Path, Query, Redirect, Session};

    pub async fn callback(
        session: Session,
        Path(provider): Path<String>,
        Query(AuthzResp {
            code,
            state: new_state,
        }): Query<AuthzResp>,
        State(config): State<Config>,
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        debug!("OAuth callback for provider: {}", provider);

        let old_state: Option<String> = session
            .get(CSRF_STATE_KEY)
            .await
            .map_err(|_| Error::SessionStateError("Failed to retrieve CSRF state".to_string()))?
            .ok_or(Error::MissingCSRFState)?;

        // Ensure the CSRF state has not been tampered with.
        if old_state != Some(new_state.secret().to_string()) {
            return Err(Error::InvalidCSRFState);
        }

        match session.remove::<String>(NEXT_URL_KEY).await {
            Ok(Some(next)) if !next.is_empty() => {
                // Parse the next URL and append the code parameter
                if let Ok(mut url) = Url::parse(&next) {
                    url.query_pairs_mut().append_pair("code", &code);
                    Ok(Redirect::to(url.as_str()).into_response())
                } else {
                    // If parsing the URL fails, log the error and redirect to the default login page
                    error!("Invalid URL in session: {}", next);
                    Ok(
                        Redirect::to(&config.website.pages.with_root().public_login)
                            .into_response(),
                    )
                }
            }
            Ok(Some(_) | None) => {
                Ok(Redirect::to(&config.website.pages.with_root().public_login).into_response())
            }
            Err(e) => {
                error!("Session error: {:?}", e);
                Err(Error::SessionError(
                    "Failed to retrieve next URL from session".to_string(),
                ))
            }
        }
    }
}
