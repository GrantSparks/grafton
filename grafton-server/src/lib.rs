mod error;
pub mod model;
mod server;
mod util;
mod web;

#[cfg(feature = "rbac")]
mod rbac;

pub use {
    error::AppError,
    server::ServerBuilder,
    tracing,
    util::{ClientConfig, Config, TracingLogger},
};
