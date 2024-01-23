#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod app;
mod error;
pub mod model;
mod util;
mod web;

mod core;
pub use core::*;

use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "rbac")]
mod rbac;

pub use {
    app::Builder,
    axum_login::axum,
    error::Error,
    tracing,
    util::{Config, Logger},
};

pub trait ServerConfigProvider:
    'static + Send + Sync + DeserializeOwned + Serialize + std::fmt::Debug
{
    fn get_server_config(&self) -> &Config;
}
