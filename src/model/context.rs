use std::{
    fmt::{Debug, Formatter},
    sync::{Arc, Mutex},
};

#[cfg(feature = "rbac")]
use oso::Oso;

use super::User;
use crate::util::{AppError, Config};

#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<Config>,

    #[cfg(feature = "rbac")]
    pub oso: Arc<Mutex<Oso>>,
}

#[cfg(feature = "rbac")]
impl Debug for AppContext {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .field("oso", &"CustomOsoDebugInfo")
            .finish()
    }
}

#[cfg(not(feature = "rbac"))]
impl Debug for AppContext {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .finish()
    }
}

impl AppContext {
    pub fn new(config: Config, #[cfg(feature = "rbac")] oso: Oso) -> Result<Self, AppError> {
        Ok(Self {
            config: Arc::new(config),
            #[cfg(feature = "rbac")]
            oso: Arc::new(Mutex::new(oso)),
        })
    }

    #[cfg(feature = "rbac")]
    pub fn is_allowed(&self, actor: User, action: &str, resource: &str) -> Result<bool, AppError> {
        let guard = self.oso.lock()?;

        guard
            .is_allowed(actor, action.to_string(), resource)
            .map_err(AppError::from)
    }
}
