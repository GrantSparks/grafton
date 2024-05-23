use {askama::Template, askama_axum::IntoResponse};

use crate::{
    axum::{
        extract::Path,
        routing::{get, post},
    },
    core::AxumRouter,
    oauth2::NextUrl,
    tracing::error,
    Config, ServerConfigProvider,
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {
    pub message: Option<String>,
    pub next: String,
    pub provider_name: String,
}

pub fn router<C>() -> AxumRouter<C>
where
    C: ServerConfigProvider,
{
    AxumRouter::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
}

mod post {

    use axum_login::tower_sessions::Session;

    use crate::{
        axum::{extract::State, response::Redirect, Form},
        oauth2::{CSRF_STATE_KEY, NEXT_URL_KEY},
        AuthSession, Error,
    };

    use super::{error, Config, IntoResponse, NextUrl, Path};

    pub async fn login(
        auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        State(_config): State<Config>,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> Result<impl IntoResponse, Error> {
        let (url, token) = auth_session
            .backend
            .authorize_url(provider.clone())
            .map_err(|e| {
                error!("Error generating authorization URL: {:?}", e);
                Error::AuthorizationUrlError(e.to_string())
            })?;

        session
            .insert(CSRF_STATE_KEY, token.secret())
            .await
            .map_err(|e| {
                error!("Error serializing CSRF token: {:?}", e);
                Error::SerializationError(e.to_string())
            })?;

        if next.is_empty() {
            error!("NEXT_URL_KEY is empty or null");
        }

        session.insert(NEXT_URL_KEY, next).await.map_err(|e| {
            error!("Error serializing next URL: {:?}", e);
            Error::SerializationError(e.to_string())
        })?;

        Ok(Redirect::to(url.as_str()).into_response())
    }
}

mod get {

    use crate::{
        axum::extract::{Query, State},
        Error,
    };

    use super::{Config, IntoResponse, Login, NextUrl, Path};

    pub async fn login(
        Query(NextUrl { next }): Query<NextUrl>,
        Path(provider): Path<String>,
        State(config): State<Config>,
    ) -> Result<Login, impl IntoResponse> {
        config.oauth_providers.get(&provider).map_or_else(
            || Err(Error::ProviderNotFoundError(provider)),
            |client| {
                let provider_name = &client.display_name;
                Ok(Login {
                    message: None,
                    next,
                    provider_name: provider_name.clone(),
                })
            },
        )
    }
}
