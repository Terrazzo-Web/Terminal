#![cfg(feature = "port-forward")]

use futures::Stream;
use terrazzo::declare_trait_aliias;
use tonic::Status;

use crate::backend::protos::terrazzo::portforward::PortForwardDataRequest;

pub mod bind;
pub mod download;
mod grpc;
mod listeners;

declare_trait_aliias!(
    RequestDataStream,
    Stream<Item = Result<PortForwardDataRequest, Status>> + Unpin + Send + 'static);
