use std::{error::Error, fmt};

#[cfg(feature = "rbac")]
use std::sync::{MutexGuard, PoisonError};

#[cfg(feature = "rbac")]
use oso::{Oso, OsoError};

#[cfg(feature = "grpc")]
use tonic::{transport::Error as TonicTransportError, Status};

#[derive(Debug)]
pub enum AppError {
    Generic(Box<dyn Error + Send + Sync>),
    #[cfg(feature = "grpc")]
    Grpc(Status),
    #[cfg(feature = "rbac")]
    OsoError(OsoError),
    PathError(String),
    #[cfg(feature = "grpc")]
    TonicTransport(TonicTransportError),
    MutexLockError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Generic(err) => write!(f, "Generic error: {}", err),
            #[cfg(feature = "grpc")]
            AppError::Grpc(status) => write!(f, "gRPC error: {}", status),
            #[cfg(feature = "rbac")]
            AppError::OsoError(err) => write!(f, "Oso error: {}", err),
            AppError::PathError(err) => write!(f, "Path error: {}", err),
            #[cfg(feature = "grpc")]
            AppError::TonicTransport(err) => write!(f, "Tonic transport error: {}", err),
            AppError::MutexLockError(err) => write!(f, "Mutex lock error: {}", err),
        }
    }
}

impl Error for AppError {}

// Automatic error type conversion
impl From<Box<dyn Error + Send + Sync>> for AppError {
    fn from(err: Box<dyn Error + Send + Sync>) -> Self {
        AppError::Generic(err)
    }
}

#[cfg(feature = "rbac")]
impl From<OsoError> for AppError {
    fn from(err: OsoError) -> Self {
        AppError::OsoError(err)
    }
}

#[cfg(feature = "grpc")]
impl From<Status> for AppError {
    fn from(status: Status) -> Self {
        AppError::Grpc(status)
    }
}

#[cfg(feature = "grpc")]
impl From<AppError> for Status {
    fn from(error: AppError) -> Self {
        match error {
            AppError::Grpc(status) => status,
            #[cfg(feature = "rbac")]
            AppError::OsoError(err) => Status::internal(format!("Oso error: {}", err)),
            AppError::Generic(err) => Status::internal(format!("Generic error: {}", err)),
            AppError::PathError(err) => Status::internal(format!("Path error: {}", err)),
            AppError::TonicTransport(err) => {
                Status::internal(format!("Tonic transport error: {}", err))
            }
            AppError::MutexLockError(err) => Status::internal(format!("Mutex lock error: {}", err)),
        }
    }
}

#[cfg(feature = "grpc")]
impl From<TonicTransportError> for AppError {
    fn from(err: TonicTransportError) -> Self {
        AppError::TonicTransport(err)
    }
}

#[cfg(feature = "rbac")]
impl From<PoisonError<MutexGuard<'_, Oso>>> for AppError {
    fn from(err: PoisonError<MutexGuard<'_, Oso>>) -> Self {
        AppError::MutexLockError(format!("Failed to acquire mutex lock: {}", err))
    }
}
