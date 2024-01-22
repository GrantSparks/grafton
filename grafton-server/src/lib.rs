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
    util::{load_config_from_dir, ClientConfig, GraftonConfig, Logger},
};

pub trait GraftonConfigProvider:
    'static + Send + Sync + DeserializeOwned + Serialize + std::fmt::Debug
{
    fn get_grafton_config(&self) -> &GraftonConfig;
}
