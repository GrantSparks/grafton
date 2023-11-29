use std::{collections::HashMap, sync::Arc};

use {
    askama_axum::IntoResponse,
    axum::{middleware::Next, response::Redirect},
    axum_login::{
        axum::{error_handling::HandleErrorLayer, http::StatusCode, middleware::from_fn, BoxError},
        tower_sessions::{MemoryStore, SessionManagerLayer},
        urlencoding, AuthManagerLayerBuilder,
    },
    oauth2::{basic::BasicClient, AuthUrl, TokenUrl},
    sqlx::SqlitePool,
    tower::ServiceBuilder,
    tracing::debug,
};

use super::auth::{AuthSession, Backend};
use crate::{
    error::AppError,
    model::AppContext,
    web::{auth, oauth, protected},
};

pub struct App {
    db: SqlitePool,
    oauth_clients: HashMap<String, BasicClient>,
    session_layer: SessionManagerLayer<MemoryStore>,
    login_url: String,
}

impl App {
    pub async fn new(
        app_ctx: Arc<AppContext>,
        session_layer: SessionManagerLayer<MemoryStore>,
    ) -> Result<Self, AppError> {
        let mut oauth_clients = HashMap::new();

        for (client_name, client_config) in app_ctx.config.oauth_clients.iter() {
            debug!("Configuring oauth client: {}", client_name);
            let client_id = client_config.client_id.clone();
            let client_secret = client_config.client_secret.clone();

            let auth_url = AuthUrl::new(client_config.auth_uri.clone()).map_err(|e| {
                AppError::InvalidAuthUrlDetailed {
                    url: client_config.auth_uri.clone(),
                    inner: e,
                    client_name: client_name.clone(),
                }
            })?;
            let token_url = TokenUrl::new(client_config.token_uri.clone()).map_err(|e| {
                AppError::InvalidTokenUrlDetailed {
                    url: client_config.token_uri.clone(),
                    inner: e,
                    client_name: client_name.clone(),
                }
            })?;

            let client =
                BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url));
            oauth_clients.insert(client_name.clone(), client);
        }

        let db = SqlitePool::connect(":memory:").await.map_err(|e| {
            AppError::DatabaseConnectionErrorDetailed {
                conn_str: ":memory:".to_string(),
                inner: e,
            }
        })?;

        sqlx::migrate!()
            .run(&db)
            .await
            .map_err(|e| AppError::DatabaseMigrationErrorDetailed {
                migration_details: "Initial migration".to_string(),
                inner: e,
            })?;

        Ok(Self {
            db,
            oauth_clients,
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
        let backend = Backend::new(self.db.clone(), self.oauth_clients.clone());
        let auth_service = ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::BAD_REQUEST
            }))
            .layer(AuthManagerLayerBuilder::new(backend, self.session_layer).build());

        let login_url = Arc::new(self.login_url);
        let auth_middleware = from_fn(move |auth_session: AuthSession, req, next: Next| {
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
