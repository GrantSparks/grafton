mod app;
mod error;
pub mod model;
mod util;
mod web;

mod r#type;
pub use r#type::*;

#[cfg(feature = "rbac")]
mod rbac;

pub use {
    app::ServerBuilder,
    error::AppError,
    tracing,
    util::{ClientConfig, Config, TracingLogger},
};
