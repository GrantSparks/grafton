#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod app;
mod error;
pub mod model;
mod util;
mod web;

mod core;
pub use core::*;

#[cfg(feature = "rbac")]
mod rbac;

pub use {
    app::Builder,
    axum_login::axum,
    error::Error,
    tracing,
    util::{ClientConfig, Config, Logger, PluginInfo},
};
