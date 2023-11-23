use std::sync::Arc;

use askama_axum::IntoResponse;
use axum::{middleware::Next, response::Redirect};
use axum_login::{
    axum::{error_handling::HandleErrorLayer, middleware::from_fn, BoxError},
    http::StatusCode,
    tower_sessions::{MemoryStore, SessionManagerLayer},
    urlencoding, AuthManagerLayerBuilder,
};
use oauth2::{basic::BasicClient, AuthUrl, TokenUrl};
use sqlx::SqlitePool;
use tower::ServiceBuilder;

use super::auth::{AuthSession, Backend};
use crate::{
    error::AppError,
    model::AppContext,
    web::{auth, oauth, protected},
};

pub struct App {
    db: SqlitePool,
    client: BasicClient,
    session_layer: SessionManagerLayer<MemoryStore>,
    login_url: String,
}

impl App {
    pub async fn new(
        app_ctx: Arc<AppContext>,
        session_layer: SessionManagerLayer<MemoryStore>,
    ) -> Result<Self, AppError> {
        let github_client = app_ctx
            .config
            .oauth_clients
            .get("github")
            .ok_or(AppError::ClientConfigNotFound("github".to_string()))?;

        let client_id = github_client.client_id.clone();
        let client_secret = github_client.client_secret.clone();

        let auth_url = AuthUrl::new(github_client.auth_uri.clone())
            .map_err(|e| AppError::InvalidAuthUrl(e.to_string()))?;
        let token_url = TokenUrl::new(github_client.token_uri.clone())
            .map_err(|e| AppError::InvalidTokenUrl(e.to_string()))?;

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));

        let db = SqlitePool::connect(":memory:")
            .await
            .map_err(|e| AppError::DatabaseConnectionError(e.to_string()))?;

        sqlx::migrate!()
            .run(&db)
            .await
            .map_err(|e| AppError::DatabaseMigrationError(e.to_string()))?;

        Ok(Self {
            db,
            client,
            session_layer,
            login_url: app_ctx
                .config
                .website
                .pages
                .with_root()
                .public_login
                .clone(),
        })
    }

    pub fn create_auth_router(self) -> axum_login::axum::Router<Arc<AppContext>> {
        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db.clone(), self.client.clone());
        let auth_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::BAD_REQUEST
            }))
            .layer(AuthManagerLayerBuilder::new(backend, self.session_layer).build());

        let login_url = Arc::new(self.login_url);
        let auth_middleware = from_fn(move |auth_session: AuthSession, req, next: Next<_>| {
            let login_url_clone = login_url.clone();
            async move {
                if auth_session.user.is_some() {
                    next.run(req).await
                } else {
                    let uri = req.uri().to_string();
                    let next = urlencoding::encode(&uri);
                    let redirect_url = format!("{}?next={}", login_url_clone, next);
                    Redirect::temporary(&redirect_url).into_response()
                }
            }
        });

        protected::router()
            .route_layer(auth_middleware)
            .merge(auth::router())
            .merge(oauth::router())
            .layer(auth_service)
    }
}
