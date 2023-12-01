use std::sync::Arc;

use super::AppContext;

pub type AxumRouter = axum_login::axum::Router<Arc<AppContext>>;
