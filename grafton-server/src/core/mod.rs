use std::sync::Arc;

use crate::{model::AppContext, web::Backend};

pub type AxumRouter = axum_login::axum::Router<Arc<AppContext>>;

pub(crate) type AuthSession = axum_login::AuthSession<Backend>;
