//! Adapt Rust errors to gRPC status errors.

use std::convert::Infallible;
use std::ops::Deref;

use tonic::Code;
use tonic::Status;

/// A trait implemented by errors that translate to gRPC status codes.
pub trait IsGrpcError: std::error::Error + Sized {
    fn code(&self) -> Code;
}

impl<T: IsGrpcError> IsGrpcError for Box<T> {
    fn code(&self) -> Code {
        self.as_ref().code()
    }
}

impl IsGrpcError for Infallible {
    fn code(&self) -> Code {
        unreachable!()
    }
}

/// A wrapper to translate errors into grpc [Status]es.
#[derive(thiserror::Error, Debug, Clone)]
#[error(transparent)]
pub struct GrpcError<E: IsGrpcError>(#[from] E);

impl<E: IsGrpcError> From<GrpcError<E>> for Status {
    fn from(error: GrpcError<E>) -> Self {
        Status::new(error.code(), error.to_string())
    }
}

impl<E: IsGrpcError> Deref for GrpcError<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
