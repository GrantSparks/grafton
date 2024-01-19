use std::{io, sync::MutexGuard, sync::PoisonError};

use {
    crate::axum::{
        body::Body,
        http::{Response as HttpResponse, StatusCode},
        response::{IntoResponse, Response},
    },
    oauth2::{basic::BasicRequestTokenError, reqwest::AsyncHttpClientError},
    sqlx::migrate::MigrateError,
    thiserror::Error,
    tokio_rustls::rustls::Error as RustlsError,
    url::ParseError,
};

#[cfg(feature = "rbac")]
use oso::{Oso, OsoError};

#[cfg(feature = "grpc")]
use tonic::{transport::Error as TonicTransportError, Status};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Generic error: {0}")]
    GenericError(String),

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

    #[error("OAuth2 generic error: {0}")]
    OAuth2Generic(String),

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

    #[error("Invalid HTTP header value: {0}")]
    InvalidHttpHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("TLS configuration error: {0}")]
    TlsConfigError(#[from] RustlsError),

    #[error("Invalid certificate")]
    InvalidCertificate,

    #[error("No private keys found in key file '{file_path}': {error}")]
    NoPrivateKey { file_path: String, error: String },

    #[error("Invalid private key in file '{file_path}': {error}")]
    InvalidPrivateKey { file_path: String, error: String },

    #[error("Session state error: {0}")]
    SessionStateError(String),

    #[error("Missing CSRF state in the session")]
    MissingCSRFState,

    #[error("Invalid CSRF state")]
    InvalidCSRFState,

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Provider not found: {0}")]
    ProviderNotFoundError(String),

    #[error("Login error: {0}")]
    LoginError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Failed to serialize session data: {0}")]
    SerializationError(String),

    #[error("Failed to generate authorization URL: {0}")]
    AuthorizationUrlError(String),

    #[error("Error rendering template: {0}")]
    TemplateRenderingError(String),
}

#[cfg(feature = "rbac")]
impl From<PoisonError<MutexGuard<'_, Oso>>> for Error {
    fn from(err: PoisonError<MutexGuard<'_, Oso>>) -> Self {
        Self::MutexLockError(format!("Failed to acquire mutex lock: {err}"))
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            Self::GenericError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            Self::GeneralError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "General error occurred".to_string(),
            ),
            Self::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize session data: {msg}"),
            ),
            Self::AuthorizationUrlError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to generate authorization URL: {msg}"),
            ),
            Self::ProviderNotFoundError(msg) => (
                StatusCode::NOT_FOUND,
                format!("OAuth provider not found: {msg}"),
            ),
            Self::TemplateRenderingError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error rendering template: {msg}"),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unexpected error occurred".to_string(),
            ),
        };

        let full_message = format!("{status}: {error_message}");
        let body = Body::from(full_message);

        HttpResponse::builder().status(status).body(body).unwrap() // Safe unwrap since we're constructing a valid response
    }
}
