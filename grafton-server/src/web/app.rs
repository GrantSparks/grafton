use std::sync::Arc;

use axum_login::{
    axum::{error_handling::HandleErrorLayer, BoxError},
    http::StatusCode,
    login_required,
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayerBuilder,
};
use oauth2::{basic::BasicClient, AuthUrl, TokenUrl};
use sqlx::SqlitePool;
use tower::ServiceBuilder;

use super::auth::Backend;
use crate::{
    error::AppError,
    model::AppContext,
    web::{auth, oauth, protected},
};

pub struct App {
    db: SqlitePool,
    client: BasicClient,
    session_layer: SessionManagerLayer<MemoryStore>,
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
        })
    }

    pub async fn create_auth_router(self) -> axum_login::axum::Router<Arc<AppContext>> {
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

        protected::router()
            .route_layer(login_required!(Backend, login_url = "/login"))
            .merge(auth::router())
            .merge(oauth::router())
            .layer(auth_service)
    }
}
