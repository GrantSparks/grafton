use std::sync::Arc;

use crate::{model::Context, web::Backend};

pub type AxumRouter = crate::axum::Router<Arc<Context>>;

#[allow(clippy::redundant_pub_crate)]
pub(crate) type AuthSession = axum_login::AuthSession<Backend>;
