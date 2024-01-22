use std::sync::Arc;

use {
    tower_http::{services::ServeDir, services::ServeFile},
    tracing::{debug, error},
};

use crate::{core::AxumRouter, model::Context, web::ProtectedApp, Error, GraftonConfigProvider};

#[cfg(feature = "rbac")]
use crate::rbac;

use super::{
    middleware::file::create_file_service, middleware::session::create_session_layer,
    server::Server,
};

type RouterFactory<C> = dyn FnOnce(&Arc<Context<C>>) -> AxumRouter<C> + Send + 'static;

type FallbackServiceFactory<C> =
    dyn FnOnce(&Arc<Context<C>>) -> Result<ServeDir<ServeFile>, Error> + Send + 'static;

pub struct Builder<C>
where
    C: GraftonConfigProvider,
{
    app_ctx: Arc<Context<C>>,
    protected_router_factory: Option<Box<RouterFactory<C>>>,
    unprotected_router_factory: Option<Box<RouterFactory<C>>>,
    fallback_service_factory: Option<Box<FallbackServiceFactory<C>>>,
}

impl<C> Builder<C>
where
    C: GraftonConfigProvider,
{
    /// # Errors
    ///
    /// This function will return an error if the config is invalid or if the rbac feature is enabled and the oso policy files are invalid.
    pub fn new(config: C) -> Result<Self, Error> {
        debug!("Initializing ServerBuilder with config: {:?}", config);

        let context = {
            #[cfg(feature = "rbac")]
            {
                // You'll need to modify `rbac::initialize` and `Context::new` to accept a generic config
                let oso = rbac::initialize(config.get_grafton_config())?;
                Context::new(config, oso)
            }
            #[cfg(not(feature = "rbac"))]
            {
                Context::new(config)?
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

    #[must_use]
    pub fn with_unprotected_router<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(&Arc<Context<C>>) -> AxumRouter<C> + Send + 'static,
    {
        self.unprotected_router_factory = Some(Box::new(factory));
        self
    }

    #[must_use]
    pub fn with_protected_router<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(&Arc<Context<C>>) -> AxumRouter<C> + Send + 'static,
    {
        self.protected_router_factory = Some(Box::new(factory));
        self
    }

    #[must_use]
    pub fn with_fallback_service<F>(mut self, factory: F) -> Self
    where
        F: FnOnce(&Arc<Context<C>>) -> Result<ServeDir<ServeFile>, Error> + Send + 'static,
    {
        self.fallback_service_factory = Some(Box::new(factory));
        self
    }

    /// Build the server. Use a default protected router or fallback service if none was provided.
    ///
    /// # Errors
    ///
    /// This function will return an error if the config is invalid
    pub async fn build(self) -> Result<Server<C>, Error> {
        let app_ctx = self.app_ctx;

        let optional_protected_router = self
            .protected_router_factory
            .map(|factory| factory(&app_ctx));

        let unprotected_router = self
            .unprotected_router_factory
            .map(|factory| factory(&app_ctx));

        let session_layer = create_session_layer();

        let app =
            ProtectedApp::new(app_ctx.clone(), session_layer, optional_protected_router).await?;
        let final_protected_router = app.create_auth_router();

        let router = if let Some(unprotected_router) = unprotected_router {
            final_protected_router.merge(unprotected_router)
        } else {
            final_protected_router
        };

        let file_service = if let Some(factory) = self.fallback_service_factory {
            factory(&app_ctx)?
        } else {
            let fallback_file_path = app_ctx.config.get_grafton_config().website.web_root.clone();
            let default_serve_file = ServeFile::new(&fallback_file_path);

            create_file_service(&app_ctx).unwrap_or_else(|e| {
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
