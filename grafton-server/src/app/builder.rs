use std::sync::Arc;

use {
    tower_http::{services::ServeDir, services::ServeFile},
    tracing::{debug, error},
};

use crate::{core::AxumRouter, model::AppContext, util::Config, web::ProtectedApp, AppError};

use super::{
    middleware::file::create_file_service, middleware::session::create_session_layer,
    server::Server,
};

type RouterFactory = dyn FnOnce(Arc<AppContext>) -> AxumRouter + 'static;
type FallbackServiceFactory =
    dyn FnOnce(Arc<AppContext>) -> Result<ServeDir<ServeFile>, AppError> + 'static;

pub struct ServerBuilder {
    app_ctx: Arc<AppContext>,
    protected_router_factory: Option<Box<RouterFactory>>,
    unprotected_router_factory: Option<Box<RouterFactory>>,
    fallback_service_factory: Option<Box<FallbackServiceFactory>>,
}

impl ServerBuilder {
    pub async fn new(config: Config) -> Result<Self, AppError> {
        debug!("Initializing ServerBuilder with config: {:?}", config);

        let context = {
            #[cfg(feature = "rbac")]
            {
                use crate::rbac;
                let oso = rbac::initialize(&config)?;
                AppContext::new(config, oso)?
            }
            #[cfg(not(feature = "rbac"))]
            {
                AppContext::new(config)?
            }
        };

        let context = Arc::new(context);

        Ok(Self {
            app_ctx: context,
            protected_router_factory: None,
            unprotected_router_factory: None,
            fallback_service_factory: None,
        })
    }

    pub fn with_unprotected_router<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(Arc<AppContext>) -> AxumRouter + 'static,
    {
        self.unprotected_router_factory = Some(Box::new(factory));
        self
    }
    pub fn with_protected_router<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(Arc<AppContext>) -> AxumRouter + 'static,
    {
        self.protected_router_factory = Some(Box::new(factory));
        self
    }

    pub fn with_fallback_service<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(Arc<AppContext>) -> Result<ServeDir<ServeFile>, AppError> + 'static,
    {
        self.fallback_service_factory = Some(Box::new(factory));
        self
    }

    /// Build the server. Use a default protected router and fallback service if none was provided.
    pub async fn build(self) -> Result<Server, AppError> {
        let app_ctx = self.app_ctx;

        let optional_protected_router = self
            .protected_router_factory
            .map(|factory| factory(app_ctx.clone()));
        let session_layer = create_session_layer();

        let unprotected_router = self
            .unprotected_router_factory
            .map(|factory| factory(app_ctx.clone()));

        let app =
            ProtectedApp::new(app_ctx.clone(), session_layer, optional_protected_router).await?;
        let final_protected_router = app.create_auth_router();

        let router = if let Some(unprotected_router) = unprotected_router {
            final_protected_router.merge(unprotected_router)
        } else {
            final_protected_router
        };

        let file_service = if let Some(factory) = self.fallback_service_factory {
            factory(app_ctx.clone())?
        } else {
            let fallback_file_path = app_ctx.config.website.web_root.clone();
            let default_serve_file = ServeFile::new(&fallback_file_path);

            create_file_service(app_ctx.clone()).unwrap_or_else(|e| {
                error!("Failed to build file service: {:?}", e);
                ServeDir::new(&fallback_file_path).fallback(default_serve_file)
            })
        };

        Ok(Server {
            router: router
                .with_state(app_ctx.clone())
                .fallback_service(file_service),
            config: app_ctx.config.clone(),
        })
    }
}
