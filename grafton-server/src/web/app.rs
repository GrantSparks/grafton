use std::sync::Arc;

use axum::{error_handling::HandleErrorLayer, BoxError};
use axum_login::{
    login_required,
    tower_sessions::{MemoryStore, SessionManagerLayer},
    AuthManagerLayer,
};
use http::StatusCode;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, TokenUrl};
use sqlx::SqlitePool;
use tower::ServiceBuilder;

use crate::{
    error::AppError,
    model::AppContext,
    web::{auth, oauth, protected},
};

use super::auth::Backend;

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
        // TODO:  Protect these strings in config
        let client_id = ClientId::new(app_ctx.config.oauth_clients["github"].client_id.clone());
        let client_secret =
            ClientSecret::new(app_ctx.config.oauth_clients["github"].client_secret.clone());

        let auth_url = AuthUrl::new(app_ctx.config.oauth_clients["github"].auth_uri.clone())
            .map_err(|e| AppError::InvalidAuthUrl(e.to_string()))?;
        let token_url = TokenUrl::new(app_ctx.config.oauth_clients["github"].token_uri.clone())
            .map_err(|e| AppError::InvalidTokenUrl(e.to_string()))?;
        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));
        let db = match SqlitePool::connect(":memory:").await {
            Ok(pool) => pool,
            Err(e) => {
                return Err(AppError::DatabaseConnectionError(e.to_string()));
            }
        };

        match sqlx::migrate!().run(&db).await {
            Ok(_) => (),
            Err(e) => {
                return Err(AppError::DatabaseMigrationError(e.to_string()));
            }
        }

        Ok(Self {
            db,
            client,
            session_layer,
        })
    }

    pub async fn create_auth_router(self) -> axum::Router<Arc<AppContext>> {
        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db.clone(), self.client.clone());
        let auth_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::BAD_REQUEST
            }))
            .layer(AuthManagerLayer::new(backend, self.session_layer));

        protected::router()
            .route_layer(login_required!(Backend, login_url = "/login"))
            .merge(auth::router())
            .merge(oauth::router())
            .layer(auth_service)
    }
}
