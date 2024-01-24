use std::sync::Arc;

use crate::{model::Context, web::Backend};

pub type AxumRouter<C> = crate::axum::Router<Arc<Context<C>>>;

pub type AuthSession = axum_login::AuthSession<Backend>;
