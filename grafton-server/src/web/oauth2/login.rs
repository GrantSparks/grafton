use std::sync::Arc;

use {askama::Template, askama_axum::IntoResponse, serde::Deserialize};

use crate::{
    axum::{
        extract::Path,
        routing::{get, post},
    },
    core::AxumRouter,
    tracing::error,
    ServerConfigProvider,
};

pub const NEXT_URL_KEY: &str = "auth.next-url";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: String,
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
}

mod post {

    use axum_login::tower_sessions::Session;

    use crate::{
        axum::{extract::State, response::Redirect, Form},
        model::Context,
        web::oauth2::CSRF_STATE_KEY,
        AuthSession, Error,
    };

    use super::{error, Arc, IntoResponse, NextUrl, Path, ServerConfigProvider, NEXT_URL_KEY};

    pub async fn login<C>(
        auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        State(_app_ctx): State<Arc<Context<C>>>,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> Result<impl IntoResponse, Error>
    where
        C: ServerConfigProvider,
    {
        match auth_session.backend.authorize_url(provider.clone()) {
            Ok((url, token)) => {
                if let Err(e) = session.insert(CSRF_STATE_KEY, token.secret()).await {
                    error!("Error serializing CSRF token: {:?}", e);
                    return Err(Error::SerializationError(e.to_string()));
                }

                if next.is_empty() {
                    error!("NEXT_URL_KEY is empty or null");
                }

                if let Err(e) = session.insert(NEXT_URL_KEY, next).await {
                    error!("Error serializing next URL: {:?}", e);
                    return Err(Error::SerializationError(e.to_string()));
                }

                Ok(Redirect::to(url.as_str()).into_response())
            }
            Err(e) => {
                error!("Error generating authorization URL: {:?}", e);
                Err(Error::AuthorizationUrlError(e.to_string()))
            }
        }
    }
}

mod get {

    use crate::{
        axum::extract::{Query, State},
        model::Context,
        Error,
    };

    use super::{Arc, IntoResponse, Login, NextUrl, Path, ProviderTemplate, ServerConfigProvider};

    pub async fn login<C>(
        Query(NextUrl { next }): Query<NextUrl>,
        Path(provider): Path<String>,
        State(app_ctx): State<Arc<Context<C>>>,
    ) -> Result<Login, impl IntoResponse>
    where
        C: ServerConfigProvider,
    {
        app_ctx
            .config
            .get_server_config()
            .oauth_clients
            .get(&provider)
            .map_or_else(
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

    pub async fn choose_provider<C>(
        Query(NextUrl { next }): Query<NextUrl>,
        State(app_ctx): State<Arc<Context<C>>>,
    ) -> Result<ProviderTemplate, Error>
    where
        C: ServerConfigProvider,
    {
        let providers = app_ctx
            .config
            .get_server_config()
            .oauth_clients
            .values()
            .map(|client| client.display_name.clone())
            .collect();

        Ok(ProviderTemplate {
            message: None,
            next,
            providers,
        })
    }
}
