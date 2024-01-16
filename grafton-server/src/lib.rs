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
    app::ServerBuilder,
    axum_login::axum,
    error::AppError,
    tracing,
    util::{ClientConfig, Config, PluginInfo, TracingLogger},
};
