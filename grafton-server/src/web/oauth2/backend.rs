use std::collections::HashMap;

use {
    axum_login::{AuthnBackend, UserId},
    oauth2::{
        basic::{BasicClient, BasicRequestTokenError},
        reqwest::async_http_client,
        url::Url,
        AuthorizationCode, CsrfToken, Scope, TokenResponse,
    },
    reqwest::header::{HeaderName as ReqwestHeaderName, HeaderValue},
    serde::Deserialize,
    sqlx::SqlitePool,
};

use crate::{axum::async_trait, model::User, AppError};

use super::Credentials;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct UserInfo {
    login: Option<String>,
    email: Option<String>,
    username: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Backend {
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

            let scopes: Vec<Scope> = vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
            ]
            .into_iter()
            .map(Scope::new)
            .collect();

            Ok(oauth_client
                .clone()
                .authorize_url(|| csrf_token.clone())
                .add_scopes(scopes)
                .url())
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

            let response = reqwest::Client::new()
                .get(creds.userinfo_uri)
                .header(user_agent_header, user_agent_value)
                .header(authorization_header, authorization_value)
                .send()
                .await
                .map_err(Self::Error::Reqwest)?;

            let user_info = response
                .json::<UserInfo>()
                .await
                .map_err(Self::Error::Reqwest)?;

            let login_id: String;
            match creds.provider.as_str() {
                "github" => match user_info.login {
                    Some(login) => login_id = login,
                    None => {
                        return Err(AppError::OAuth2Generic(
                            "Login not found in response from GitHub.".to_string(),
                        ))
                    }
                },
                "google" => match user_info.email {
                    Some(email) => login_id = email,
                    None => {
                        return Err(AppError::OAuth2Generic(
                            "Email not found in response from Google.".to_string(),
                        ))
                    }
                },
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
