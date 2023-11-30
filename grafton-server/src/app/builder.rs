use std::sync::Arc;

use tracing::debug;

use crate::{
    app::middleware::session::create_session_layer, model::AppContext, util::Config, web::App,
    AppError,
};

use super::server::Server;

pub struct ServerBuilder {
    pub app_ctx: Arc<AppContext>,
    pub inner_router: axum_login::axum::Router<std::sync::Arc<AppContext>>,
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

        let session_layer = create_session_layer();

        let app_result = App::new(context.clone(), session_layer).await;
        let app = match app_result {
            Ok(app) => app,
            Err(e) => return Err(e),
        };

        let inner_router = app.create_auth_router();

        debug!("ServerBuilder initialized");
        Ok(Self {
            app_ctx: context,
            inner_router,
        })
    }

    pub fn build(self) -> Result<Server, AppError> {
        debug!("Building server");

        let config = self.app_ctx.config.clone();
        let router = self.inner_router.with_state(self.app_ctx);

        debug!("Server built successfully");
        Ok(Server { router, config })
    }
}
