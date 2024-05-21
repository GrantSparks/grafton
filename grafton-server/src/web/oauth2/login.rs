use super::AuthzReq;

use {
    askama::Template,
    askama_axum::IntoResponse,
    serde::{Deserialize, Serialize},
};

use crate::{
    axum::{
        extract::Path,
        routing::{get, post},
    },
    core::AxumRouter,
    tracing::error,
    Config, ServerConfigProvider,
};

pub const NEXT_URL_KEY: &str = "auth.next-url";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OpenAiAuthParams {
    pub grant_type: String,
    pub client_id: String,
    pub client_secret: String,
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum NextOrAuthzReq {
    NextUrl(NextUrl),
    AuthzReq(AuthzReq),
}
#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {
    pub message: Option<String>,
    pub next: String,
    pub provider_name: String,
}

#[derive(Template)]
#[template(path = "provider.html")]
pub struct ProviderTemplate {
    pub message: Option<String>,
    pub next: String,
    pub providers: Vec<String>,
}

pub fn router<C>() -> AxumRouter<C>
where
    C: ServerConfigProvider,
{
    AxumRouter::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
        .route("/login", get(self::get::choose_provider))
        .route("/oauth/auth", get(self::get::choose_provider))
}

mod post {

    use axum_login::tower_sessions::Session;

    use crate::{
        axum::{extract::State, response::Redirect, Form},
        web::oauth2::CSRF_STATE_KEY,
        AuthSession, Error,
    };

    use super::{error, Config, IntoResponse, NextUrl, Path, NEXT_URL_KEY};

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
        web::oauth2::AuthzReq,
        Error,
    };

    use super::{Config, IntoResponse, Login, NextOrAuthzReq, NextUrl, Path, ProviderTemplate};

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

    pub async fn choose_provider(
        Query(next_or_authz): Query<NextOrAuthzReq>,
        State(config): State<Config>,
    ) -> Result<ProviderTemplate, Error> {
        let providers = config
            .oauth_providers
            .values()
            .map(|client| client.display_name.clone())
            .collect();

        let next = match next_or_authz {
            NextOrAuthzReq::NextUrl(NextUrl { next }) => next,
            NextOrAuthzReq::AuthzReq(AuthzReq {
                redirect_uri,
                state,
                ..
            }) => {
                let separator = if redirect_uri.contains('?') { '&' } else { '?' };
                format!("{}{}state={}", redirect_uri, separator, state.secret())
            }
        };

        Ok(ProviderTemplate {
            message: None,
            next,
            providers,
        })
    }
}
