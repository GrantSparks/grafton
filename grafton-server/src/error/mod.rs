use oauth2::{basic::BasicRequestTokenError, reqwest::AsyncHttpClientError};
#[cfg(feature = "rbac")]
use oso::{Oso, OsoError};
use sqlx::migrate::MigrateError;
use std::io;
use std::sync::MutexGuard;
use std::sync::PoisonError;
use thiserror::Error;
#[cfg(feature = "grpc")]
use tonic::{transport::Error as TonicTransportError, Status};
use url::ParseError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("General error: {0}")]
    GeneralError(#[from] anyhow::Error),

    #[cfg(feature = "grpc")]
    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),

    #[cfg(feature = "rbac")]
    #[error("Oso error: {0}")]
    OsoError(#[from] OsoError),

    #[error("Path error: {0}")]
    PathError(String),

    #[cfg(feature = "grpc")]
    #[error("Tonic transport error: {0}")]
    TonicTransport(#[from] TonicTransportError),

    #[error("Mutex lock error: {0}")]
    MutexLockError(String),

    #[error("Database connection error: {0}")]
    DatabaseConnectionError(String),

    #[error("Database migration error: {0}")]
    DatabaseMigrationError(String),

    #[error("Invalid authentication URL: {0}")]
    InvalidAuthUrl(String),

    #[error("Invalid token URL: {0}")]
    InvalidTokenUrl(String),

    #[error("Invalid config: {0}")]
    ConfigError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    #[error("Client configuration not found: {0}")]
    ClientConfigNotFound(String),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("OAuth2 error: {0}")]
    OAuth2(#[from] BasicRequestTokenError<AsyncHttpClientError>),

    #[error("Error formatting URL with protocol '{protocol}', hostname '{hostname}', port {port}, cause {cause}, inner {inner}")]
    UrlFormatError {
        protocol: String,
        hostname: String,
        port: u16,
        inner: ParseError,
        cause: String,
    },

    #[error("Invalid authentication URL for client {client_name} '{url}': {inner}")]
    InvalidAuthUrlDetailed {
        client_name: String,
        url: String,
        inner: ParseError,
    },

    #[error("Invalid token URL for {client_name} '{url}': {inner}")]
    InvalidTokenUrlDetailed {
        client_name: String,
        url: String,
        inner: ParseError,
    },

    #[error("Database connection error with '{conn_str}': {inner}")]
    DatabaseConnectionErrorDetailed {
        conn_str: String,
        inner: sqlx::Error,
    },

    #[error("Database migration error during '{migration_details}': {inner}")]
    DatabaseMigrationErrorDetailed {
        migration_details: String,
        inner: MigrateError,
    },
}

#[cfg(feature = "rbac")]
impl From<PoisonError<MutexGuard<'_, Oso>>> for AppError {
    fn from(err: PoisonError<MutexGuard<'_, Oso>>) -> Self {
        AppError::MutexLockError(format!("Failed to acquire mutex lock: {}", err))
    }
}
