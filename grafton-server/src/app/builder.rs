use std::sync::Arc;

use tracing::debug;

use crate::{
    app::middleware::session::create_session_layer,
    model::{AppContext, AxumRouter},
    util::Config,
    web::ProtectedApp,
    AppError,
};

use super::server::Server;

pub struct ServerBuilder {
    pub app_ctx: Arc<AppContext>,
    pub protected_router: Option<AxumRouter>,
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
        })
    }

    pub fn with_protected_router(mut self, router: AxumRouter) -> Self {
        self.protected_router = Some(router);
        self
    }

    pub async fn build(self) -> Result<Server, AppError> {
        let app_ctx_clone = self.app_ctx.clone();

        let router = if let Some(router) = self.protected_router {
            router.with_state(app_ctx_clone.clone())
        } else {
            let session_layer = create_session_layer();
            let app = ProtectedApp::new(app_ctx_clone.clone(), session_layer, None).await?;
            app.create_auth_router().with_state(app_ctx_clone.clone())
        };

        Ok(Server {
            router,
            config: self.app_ctx.config.clone(),
        })
    }
}
