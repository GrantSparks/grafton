#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod app;
mod error;
pub mod model;
mod util;
mod web;

mod core;
pub use core::*;

use grafton_config::TokenExpandingConfig;

#[cfg(feature = "rbac")]
mod rbac;

pub use {
    app::Builder,
    axum,
    error::Error,
    tracing,
    util::{Config, Logger, SslConfig},
};

pub trait ServerConfigProvider: TokenExpandingConfig {
    fn get_server_config(&self) -> &Config;
}
