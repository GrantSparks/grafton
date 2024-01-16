use std::sync::Arc;

use crate::{model::AppContext, web::Backend};

pub type AxumRouter = crate::axum::Router<Arc<AppContext>>;

pub(crate) type AuthSession = axum_login::AuthSession<Backend>;
