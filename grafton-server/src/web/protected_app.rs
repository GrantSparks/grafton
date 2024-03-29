use std::{collections::HashMap, sync::Arc};

use {
    askama_axum::IntoResponse,
    axum_login::{
        tower_sessions::{MemoryStore, SessionManagerLayer},
        url_with_redirect_query, AuthManagerLayerBuilder,
    },
    oauth2::{basic::BasicClient, AuthUrl, RedirectUrl, TokenUrl},
    sqlx::SqlitePool,
};

use crate::{
    axum::{
        extract::OriginalUri, http::StatusCode, middleware::from_fn, middleware::Next,
        response::Redirect,
    },
    core::AxumRouter,
    error::Error,
    model::Context,
    tracing::{debug, error, info},
    web::{
        oauth2::{create_callback_router, create_login_router, create_logout_router},
        router::protected,
        Backend,
    },
    AuthSession, ServerConfigProvider,
};

pub struct ProtectedApp<C>
where
    C: ServerConfigProvider,
{
    db: SqlitePool,
    oauth_clients: HashMap<String, BasicClient>,
    session_layer: SessionManagerLayer<MemoryStore>,
    login_url: String,
    protected_router: Option<AxumRouter<C>>,
    protected_route: String,
}

impl<C> ProtectedApp<C>
where
    C: ServerConfigProvider,
{
    pub async fn new(
        app_ctx: Arc<Context<C>>,
        session_layer: SessionManagerLayer<MemoryStore>,
        protected_router: Option<AxumRouter<C>>,
    ) -> Result<Self, Error> {
        let mut oauth_clients = HashMap::new();

        for (client_name, client_config) in &app_ctx.config.get_server_config().oauth_clients {
            debug!("Configuring oauth client: {}", client_name);
            let client_id = client_config.client_id.clone();
            let client_secret = client_config.client_secret.clone();

            let auth_url = AuthUrl::new(client_config.auth_uri.clone())?;
            let token_url = TokenUrl::new(client_config.token_uri.clone())?;

            let normalised_url = app_ctx
                .config
                .get_server_config()
                .website
                .format_public_server_url(&format!("/oauth/{client_name}/callback"));

            let redirect_url = RedirectUrl::new(normalised_url)?;

            let client =
                BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
                    .set_redirect_uri(redirect_url);

            oauth_clients.insert(client_name.clone(), client);
            debug!("OAuth client configured: {}", client_name);
        }

        let db = SqlitePool::connect(":memory:").await?;

        debug!("Running database migrations");
        sqlx::migrate!().run(&db).await?;

        info!("App successfully initialized");

        Ok(Self {
            db,
            oauth_clients,
            session_layer,
            login_url: app_ctx
                .config
                .get_server_config()
                .website
                .pages
                .with_root()
                .public_login,
            protected_router,
            protected_route: app_ctx
                .config
                .get_server_config()
                .website
                .pages
                .with_root()
                .protected_home,
        })
    }

    #[allow(clippy::cognitive_complexity)]
    pub fn create_auth_router(self) -> AxumRouter<C> {
        debug!("Creating auth router");
        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db.clone(), self.oauth_clients.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, self.session_layer).build();

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

        let router = if let Some(router) = self.protected_router {
            debug!("Using provided protected_router");
            router
        } else {
            debug!(
                "No protected_router provided, using default protected::router() at route: {}",
                self.protected_route
            );
            protected::router(&self.protected_route)
        };

        router
            .route_layer(auth_middleware)
            .merge(create_login_router())
            .merge(create_callback_router())
            .merge(create_logout_router("/logout"))
            .layer(auth_layer)
    }
}
