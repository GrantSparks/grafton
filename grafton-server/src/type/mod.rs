use std::sync::Arc;

use crate::model::AppContext;

pub type AxumRouter = axum_login::axum::Router<Arc<AppContext>>;
