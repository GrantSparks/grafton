use std::{collections::HashMap, sync::Arc};

use {
    askama_axum::IntoResponse,
    axum_login::{
        axum::{
            error_handling::HandleErrorLayer, extract::OriginalUri, http::StatusCode,
            middleware::from_fn, middleware::Next, response::Redirect, BoxError,
        },
        tower_sessions::{MemoryStore, SessionManagerLayer},
        url_with_redirect_query, AuthManagerLayerBuilder,
    },
    oauth2::{basic::BasicClient, AuthUrl, TokenUrl},
    sqlx::SqlitePool,
    tower::ServiceBuilder,
    tracing::{debug, error, info},
};

use crate::{
    core::AxumRouter,
    error::AppError,
    model::AppContext,
    web::{
        oauth2::{create_callback_router, create_login_router, create_logout_router},
        router::protected,
        Backend,
    },
    AuthSession,
};

pub struct ProtectedApp {
    db: SqlitePool,
    oauth_clients: HashMap<String, BasicClient>,
    session_layer: SessionManagerLayer<MemoryStore>,
    login_url: String,
    protected_router: Option<AxumRouter>,
    protected_route: String,
}

impl ProtectedApp {
    pub async fn new(
        app_ctx: Arc<AppContext>,
        session_layer: SessionManagerLayer<MemoryStore>,
        protected_router: Option<AxumRouter>,
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
            debug!("OAuth client configured: {}", client_name);
        }

        let db = SqlitePool::connect(":memory:").await.map_err(|e| {
            error!("Database connection error: {}", e);
            AppError::DatabaseConnectionErrorDetailed {
                conn_str: ":memory:".to_string(),
                inner: e,
            }
        })?;

        debug!("Running database migrations");
        sqlx::migrate!().run(&db).await.map_err(|e| {
            error!("Database migration error: {}", e);
            AppError::DatabaseMigrationErrorDetailed {
                migration_details: "Initial migration".to_string(),
                inner: e,
            }
        })?;

        info!("App successfully initialized");

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
            protected_router,
            protected_route: app_ctx
                .config
                .website
                .pages
                .with_root()
                .protected_home
                .clone(),
        })
    }

    pub fn create_auth_router(self) -> AxumRouter {
        debug!("Creating auth router");
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
        let auth_middleware = from_fn(
            move |auth_session: AuthSession,
                  OriginalUri(original_uri): OriginalUri,
                  req,
                  next: Next| {
                let login_url_clone = login_url.clone();
                async move {
                    if auth_session.user.is_some() {
                        next.run(req).await
                    } else {
                        match url_with_redirect_query(&login_url_clone, "next", original_uri) {
                            Ok(login_url) => {
                                Redirect::temporary(&login_url.to_string()).into_response()
                            }

                            Err(err) => {
                                error!(err = %err);
                                StatusCode::INTERNAL_SERVER_ERROR.into_response()
                            }
                        }
                    }
                }
            },
        );
        info!("Auth middleware created");

        let router = match self.protected_router {
            Some(router) => {
                debug!("Using provided protected_router");
                router
            }
            None => {
                debug!(
                    "No protected_router provided, using default protected::router() at route: {}",
                    self.protected_route
                );
                protected::router(self.protected_route)
            }
        };

        router
            .route_layer(auth_middleware)
            .merge(create_login_router())
            .merge(create_callback_router())
            .merge(create_logout_router())
            .layer(auth_service)
    }
}
