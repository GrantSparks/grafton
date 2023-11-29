use std::{collections::HashMap, sync::Arc};

use askama::Template;
use axum_login::{
    axum::{
        async_trait,
        extract::Query,
        http::StatusCode,
        response::{IntoResponse, Redirect},
        routing::{get, post},
        Form,
    },
    tower_sessions::Session,
    AuthnBackend, UserId,
};
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    reqwest::async_http_client,
    url::Url,
    AuthorizationCode, CsrfToken, TokenResponse,
};
use reqwest::header::{HeaderName as ReqwestHeaderName, HeaderValue};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    model::{AppContext, User},
    web::oauth::CSRF_STATE_KEY,
    AppError,
};

pub const NEXT_URL_KEY: &str = "auth.next-url";

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub message: Option<String>,
    pub next: Option<String>,
}

// This allows us to extract the "next" field from the query string. We use this
// to redirect after log in.
#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub fn router() -> axum_login::axum::Router<Arc<AppContext>> {
    axum_login::axum::Router::new()
        .route("/login/:provider", post(self::post::login))
        .route("/login/:provider", get(self::get::login))
        .route("/logout", get(self::get::logout))
}

mod post {
    use axum::extract::Path;

    use super::*;

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
    use super::*;

    pub async fn login(Query(NextUrl { next }): Query<NextUrl>) -> LoginTemplate {
        LoginTemplate {
            message: None,
            next,
        }
    }

    pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
        match auth_session.logout() {
            Ok(_) => Redirect::to("/login/github").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
    pub provider: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    login: String,
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: SqlitePool,
    oauth_clients: HashMap<String, BasicClient>,
}

impl Backend {
    pub fn new(db: SqlitePool, oauth_clients: HashMap<String, BasicClient>) -> Self {
        Self { db, oauth_clients }
    }

    pub fn authorize_url(&self, provider: String) -> Result<(Url, CsrfToken), AppError> {
        if let Some(oauth_client) = self.oauth_clients.get(&provider) {
            let csrf_token = CsrfToken::new_random();
            Ok(oauth_client.authorize_url(|| csrf_token.clone()).url())
        } else {
            Err(AppError::ClientConfigNotFound(provider))
        }
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = AppError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // Ensure the CSRF state has not been tampered with.
        if creds.old_state.secret() != creds.new_state.secret() {
            return Ok(None);
        };

        if let Some(oauth_client) = self.oauth_clients.get(&creds.provider) {
            // Use oauth_client for the token exchange
            let token_res = oauth_client
                .exchange_code(AuthorizationCode::new(creds.code))
                .request_async(async_http_client)
                .await
                .map_err(Self::Error::OAuth2)?;

            let user_agent_header = ReqwestHeaderName::from_static("user-agent");
            let authorization_header = ReqwestHeaderName::from_static("authorization");

            let user_agent_value = HeaderValue::from_static("axum-login");
            let authorization_value =
                HeaderValue::from_str(&format!("Bearer {}", token_res.access_token().secret()))
                    .map_err(AppError::InvalidHttpHeaderValue)?;

            let login_id;
            match creds.provider.as_str() {
                "github" => {
                    let user_info = reqwest::Client::new()
                        .get("https://api.github.com/user")
                        .header(user_agent_header, user_agent_value)
                        .header(authorization_header, authorization_value)
                        .send()
                        .await
                        .map_err(Self::Error::Reqwest)?
                        .json::<UserInfo>()
                        .await
                        .map_err(Self::Error::Reqwest)?;

                    login_id = user_info.login;
                }
                "google" => {
                    let user_info = reqwest::Client::new()
                        .get("https://www.googleapis.com/oauth2/v3/userinfo")
                        .header(authorization_header, authorization_value)
                        .send()
                        .await
                        .map_err(Self::Error::Reqwest)?
                        .json::<UserInfo>()
                        .await
                        .map_err(Self::Error::Reqwest)?;

                    login_id = user_info.login;
                }
                _ => {
                    return Err(AppError::OAuth2(BasicRequestTokenError::Other(format!(
                        "Unsupported provider `{}`.",
                        creds.provider
                    ))))
                }
            }

            // Persist user in our database so we can use `get_user`.
            let user = sqlx::query_as(
                r#"
            insert into users (username, access_token)
            values (?, ?)
            on conflict(username) do update
            set access_token = excluded.access_token
            returning *
            "#,
            )
            .bind(login_id)
            .bind(token_res.access_token().secret())
            .fetch_one(&self.db)
            .await
            .map_err(Self::Error::Sqlx)?;

            Ok(Some(user))
        } else {
            return Err(AppError::ClientConfigNotFound(creds.provider));
        }
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(sqlx::query_as("select * from users where id = ?")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(Self::Error::Sqlx)?)
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;
