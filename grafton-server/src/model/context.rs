use std::{
    fmt::{Debug, Formatter},
    sync::{Arc, Mutex},
};

#[cfg(feature = "rbac")]
use oso::Oso;

use super::User;
use crate::{error::Error, util::Config};

#[derive(Clone)]
pub struct Context {
    pub config: Arc<Config>,

    #[cfg(feature = "rbac")]
    pub oso: Arc<Mutex<Oso>>,
}

#[cfg(feature = "rbac")]
impl Debug for Context {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .field("oso", &"CustomOsoDebugInfo")
            .finish()
    }
}

#[cfg(not(feature = "rbac"))]
impl Debug for Context {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .finish()
    }
}

impl Context {
    #[must_use]
    pub fn new(config: Config, #[cfg(feature = "rbac")] oso: Oso) -> Self {
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
