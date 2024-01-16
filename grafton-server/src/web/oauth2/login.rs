use {askama::Template, askama_axum::IntoResponse, serde::Deserialize};

use crate::{
    axum::{
        extract::Path,
        routing::{get, post},
    },
    core::AxumRouter,
    tracing::error,
};

pub const NEXT_URL_KEY: &str = "auth.next-url";

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: String,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
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

pub fn router() -> AxumRouter {
    AxumRouter::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
        .route("/login", get(self::get::choose_provider))
}

mod post {
    use std::sync::Arc;

    use axum_login::tower_sessions::Session;

    use crate::{
        axum::{extract::State, response::Redirect, Form},
        model::AppContext,
        web::oauth2::CSRF_STATE_KEY,
        AppError, AuthSession,
    };

    use super::{error, IntoResponse, NextUrl, Path, NEXT_URL_KEY};

    /// Redirects to the OAuth2 provider's authorization URL.
    pub async fn login(
        auth_session: AuthSession,
        session: Session,
        Path(provider): Path<String>,
        State(_app_ctx): State<Arc<AppContext>>,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> Result<impl IntoResponse, AppError> {
        match auth_session.backend.authorize_url(provider.clone()) {
            Ok((url, token)) => {
                if let Err(e) = session.insert(CSRF_STATE_KEY, token.secret()).await {
                    error!("Error serializing CSRF token: {:?}", e);
                    return Err(AppError::SerializationError(e.to_string()));
                }

                if next.is_empty() {
                    error!("NEXT_URL_KEY is empty or null");
                }

                if let Err(e) = session.insert(NEXT_URL_KEY, next).await {
                    error!("Error serializing next URL: {:?}", e);
                    return Err(AppError::SerializationError(e.to_string()));
                }

                Ok(Redirect::to(url.as_str()).into_response())
            }
            Err(e) => {
                error!("Error generating authorization URL: {:?}", e);
                Err(AppError::AuthorizationUrlError(e.to_string()))
            }
        }
    }
}

mod get {

    use std::sync::Arc;

    use crate::{
        axum::extract::{Query, State},
        model::AppContext,
        AppError,
    };

    use super::{IntoResponse, LoginTemplate, NextUrl, Path, ProviderTemplate};

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
            None => Err(AppError::ProviderNotFoundError(provider)),
        }
    }

    pub async fn choose_provider(
        Query(NextUrl { next }): Query<NextUrl>,
        State(app_ctx): State<Arc<AppContext>>,
    ) -> Result<ProviderTemplate, AppError> {
        let providers = app_ctx
            .config
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
