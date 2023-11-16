use std::sync::Arc;

use askama::Template;
use axum_login::{
    axum,
    axum::{
        async_trait,
        extract::Query,
        response::{IntoResponse, Redirect},
        routing::{get, post},
        Form,
    },
    http::{
        header::{AUTHORIZATION, USER_AGENT},
        StatusCode,
    },
    tower_sessions::Session,
    AuthnBackend, UserId,
};
use oauth2::{
    basic::{BasicClient, BasicRequestTokenError},
    reqwest::{async_http_client, AsyncHttpClientError},
    url::Url,
    AuthorizationCode, CsrfToken, TokenResponse,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::{
    model::{AppContext, User},
    web::oauth::CSRF_STATE_KEY,
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

pub fn router() -> axum::Router<Arc<AppContext>> {
    axum::Router::new()
        .route("/login", post(self::post::login))
        .route("/login", get(self::get::login))
        .route("/logout", get(self::get::logout))
}

mod post {
    use super::*;

    pub async fn login(
        auth_session: AuthSession,
        session: Session,
        Form(NextUrl { next }): Form<NextUrl>,
    ) -> impl IntoResponse {
        let (auth_url, csrf_state) = auth_session.backend.authorize_url();

        session
            .insert(CSRF_STATE_KEY, csrf_state.secret())
            .expect("Serialization should not fail.");

        session
            .insert(NEXT_URL_KEY, next)
            .expect("Serialization should not fail.");

        Redirect::to(auth_url.as_str()).into_response()
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
            Ok(_) => Redirect::to("/login").into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub old_state: CsrfToken,
    pub new_state: CsrfToken,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    login: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error(transparent)]
    Sqlx(sqlx::Error),

    #[error(transparent)]
    Reqwest(reqwest::Error),

    #[error(transparent)]
    OAuth2(BasicRequestTokenError<AsyncHttpClientError>),
}

#[derive(Debug, Clone)]
pub struct Backend {
    db: SqlitePool,
    client: BasicClient,
}

impl Backend {
    pub fn new(db: SqlitePool, client: BasicClient) -> Self {
        Self { db, client }
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.client.authorize_url(CsrfToken::new_random).url()
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = BackendError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // Ensure the CSRF state has not been tampered with.
        if creds.old_state.secret() != creds.new_state.secret() {
            return Ok(None);
        };

        // Process authorization code, expecting a token response back.
        let token_res = self
            .client
            .exchange_code(AuthorizationCode::new(creds.code))
            .request_async(async_http_client)
            .await
            .map_err(Self::Error::OAuth2)?;

        // Use access token to request user info.
        let user_info = reqwest::Client::new()
            .get("https://api.github.com/user")
            .header(USER_AGENT, "axum-login") // See: https://docs.github.com/en/rest/overview/resources-in-the-rest-api?apiVersion=2022-11-28#user-agent-required
            .header(
                AUTHORIZATION,
                format!("Bearer {}", token_res.access_token().secret()),
            )
            .send()
            .await
            .map_err(Self::Error::Reqwest)?
            .json::<UserInfo>()
            .await
            .map_err(Self::Error::Reqwest)?;

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
        .bind(user_info.login)
        .bind(token_res.access_token().secret())
        .fetch_one(&self.db)
        .await
        .map_err(Self::Error::Sqlx)?;

        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(sqlx::query_as("select * from users where id = ?")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(Self::Error::Sqlx)?)
    }
}

// We use a type alias for convenience.
//
// Note that we've supplied our concrete backend here.
pub type AuthSession = axum_login::AuthSession<Backend>;
