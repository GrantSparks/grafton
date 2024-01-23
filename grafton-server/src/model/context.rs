use std::{
    fmt::{Debug, Formatter},
    sync::{Arc, Mutex},
};

use axum_login::axum::extract::FromRef;

#[cfg(feature = "rbac")]
use oso::Oso;

use crate::{error::Error, ServerConfigProvider};

use super::User;

#[derive(Clone)]
pub struct Context<C>
where
    C: ServerConfigProvider,
{
    pub config: Arc<C>,

    #[cfg(feature = "rbac")]
    pub oso: Arc<Mutex<Oso>>,
}

#[cfg(feature = "rbac")]
impl<C> Debug for Context<C>
where
    C: ServerConfigProvider,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .field("oso", &"CustomOsoDebugInfo")
            .finish()
    }
}

#[cfg(not(feature = "rbac"))]
impl<C> Debug for Context<C>
where
    C: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .finish()
    }
}

impl<C> Context<C>
where
    C: ServerConfigProvider,
{
    #[must_use]
    pub fn new(config: C, #[cfg(feature = "rbac")] oso: Oso) -> Self {
        Self {
            config: Arc::new(config),
            #[cfg(feature = "rbac")]
            oso: Arc::new(Mutex::new(oso)),
        }
    }

    /// # Errors
    ///
    /// This function will return an error if the oso policy files are invalid.
    #[cfg(feature = "rbac")]
    pub fn is_allowed(&self, actor: User, action: &str, resource: &str) -> Result<bool, Error> {
        let guard = self.oso.lock()?;

        guard
            .is_allowed(actor, action.to_string(), resource)
            .map_err(Error::from)
    }
}

#[allow(clippy::clone_on_copy)]
impl<C> FromRef<Context<C>> for Arc<C>
where
    C: ServerConfigProvider,
{
    fn from_ref(state: &Context<C>) -> Self {
        state.config.clone()
    }
}
