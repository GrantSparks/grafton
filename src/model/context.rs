// Core Rust modules
use std::sync::{Arc, Mutex};

use oso::Oso;
use tracing::error;

use super::User;
use crate::util::config::Config;

#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<Config>,
    pub oso: Arc<Mutex<Oso>>,
}

impl std::fmt::Debug for AppContext {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("AppContext")
            .field("config", &self.config)
            .field("oso", &"CustomOsoDebugInfo")
            .finish()
    }
}

impl AppContext {
    pub fn new(
        config: Arc<Config>,
        oso: Arc<Mutex<Oso>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self { config, oso })
    }

    pub fn is_allowed(&self, actor: User, action: &str, resource: &str) -> bool {
        let guard = match self.oso.lock() {
            Ok(g) => g,
            Err(e) => {
                error!("Failed to lock oso: {}", e);
                return false;
            }
        };

        match guard.is_allowed(actor, action.to_string(), resource) {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to check if action is allowed: {}", e);
                false
            }
        }
    }
}
