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

pub struct ServerBuilder {
    pub app_ctx: Arc<AppContext>,
    pub protected_router: Option<AxumRouter>,
    pub fallback_service: Option<ServeDir<ServeFile>>,
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
            protected_router: None,
            fallback_service: None,
        })
    }

    pub fn with_protected_router(mut self, router: AxumRouter) -> Self {
        self.protected_router = Some(router);
        self
    }

    pub fn with_fallback_service(mut self, fallback_service: ServeDir<ServeFile>) -> Self {
        self.fallback_service = Some(fallback_service);
        self
    }

    /// Build the server. Use a default protected router and fallback service if none was provided.
    pub async fn build(self) -> Result<Server, AppError> {
        let app_ctx = self.app_ctx;

        let protected_router = self
            .protected_router
            .map(|router| router.with_state(app_ctx.clone()));

        let router = if let Some(router) = protected_router {
            router
        } else {
            let session_layer = create_session_layer();
            let app = ProtectedApp::new(app_ctx.clone(), session_layer, None).await?;
            app.create_auth_router().with_state(app_ctx.clone())
        };

        let file_service = if let Some(fallback_service) = self.fallback_service {
            fallback_service
        } else {
            let fallback_file_path = app_ctx.config.website.web_root.clone();
            let default_serve_file = ServeFile::new(fallback_file_path.clone());

            create_file_service(app_ctx.clone()).unwrap_or_else(|e| {
                error!("Failed to build file service: {:?}", e);
                ServeDir::new(&fallback_file_path).fallback(default_serve_file)
            })
        };

        Ok(Server {
            router: router.fallback_service(file_service),
            config: app_ctx.config.clone(),
        })
    }
}
