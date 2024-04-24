use axum_login::tower_sessions::Session;

use crate::{
    axum::{
        extract::{Path, Query},
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::get,
    },
    core::AxumRouter,
    tracing::{debug, error, warn},
    ServerConfigProvider,
};

pub fn router<C>() -> AxumRouter<C>
where
    C: ServerConfigProvider,
{
    AxumRouter::new().route("/oauth/:provider/callback", get(self::get::callback))
}

mod get {
    use crate::{
        axum::extract::State,
        web::{
            oauth2::{
                login::{ProviderTemplate, NEXT_URL_KEY},
                AuthzResp, CSRF_STATE_KEY,
            },
            Credentials,
        },
        AuthSession, Config, Error,
    };

    use super::{debug, error, warn, IntoResponse, Path, Query, Redirect, Session, StatusCode};

    pub async fn callback(
        mut auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        Query(AuthzResp {
            code,
            state: new_state,
        }): Query<AuthzResp>,
        State(config): State<Config>,
    ) -> Result<impl IntoResponse, impl IntoResponse> {
        debug!("OAuth callback for provider: {}", provider);

        let old_state = session
            .get(CSRF_STATE_KEY)
            .await
            .map_err(|_| Error::SessionStateError("Failed to retrieve CSRF state".to_string()))?
            .ok_or(Error::MissingCSRFState)?;

        if let Some(oauth_client) = config.oauth_providers.get(&provider) {
            if let Some(userinfo_uri) = oauth_client.extra.get("userinfo_uri") {
                let userinfo_uri = userinfo_uri.as_str().unwrap().to_string();
                let creds = Credentials {
                    code,
                    old_state,
                    new_state,
                    provider,
                    userinfo_uri,
                };

                let user = match auth_session.authenticate(creds).await {
                    Ok(Some(user)) => {
                        debug!("User authenticated successfully");
                        user
                    }
                    Ok(None) => {
                        warn!("Invalid CSRF state, authentication failed");

                        let providers = config
                            .oauth_providers
                            .values()
                            .map(|client| client.display_name.clone())
                            .collect();

                        let next = match session.get::<String>(NEXT_URL_KEY).await {
                            Ok(Some(next)) => next,
                            Ok(None) => config.website.pages.with_root().public_home,
                            Err(e) => {
                                error!("Session error: {:?}", e);
                                return Err(Error::SessionError(
                                    "Failed to retrieve next URL from session".to_string(),
                                ));
                            }
                        };

                        return Ok((
                            StatusCode::UNAUTHORIZED,
                            ProviderTemplate {
                                message: Some("Invalid CSRF state.".to_string()),
                                next,
                                providers,
                            },
                        )
                            .into_response());
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

                match session.remove::<String>(NEXT_URL_KEY).await {
                    Ok(Some(next)) if !next.is_empty() => Ok(Redirect::to(&next).into_response()),
                    Ok(Some(_) | None) => {
                        Ok(Redirect::to(&config.website.pages.with_root().public_home)
                            .into_response())
                    }
                    Err(e) => {
                        error!("Session error: {:?}", e);
                        Err(Error::SessionError(
                            "Failed to retrieve next URL from session".to_string(),
                        ))
                    }
                }
            } else {
                Err(Error::ClientConfigNotFound("userinfo_uri".to_string()))
            }
        } else {
            Err(Error::ProviderNotFoundError(provider))
        }
    }
}
