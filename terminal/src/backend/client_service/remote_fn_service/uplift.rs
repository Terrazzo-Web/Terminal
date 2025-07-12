use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::ready;

use pin_project::pin_project;
use tonic::*;
use trz_gateway_server::server::Server;

use crate::backend::client_service::remote_fn_service::RemoteFnError;

/// Helper to uplift a remote function into a String -> String server_fn.
pub const fn uplift<Req, F, Res, E>(
    function: impl Fn(&Server, Req) -> F + 'static,
) -> impl Fn(&Server, &str) -> UpliftFuture<F>
where
    Req: for<'de> serde::Deserialize<'de>,
    F: Future<Output = Result<Res, E>> + 'static,
    Res: serde::Serialize,
    Status: From<E>,
{
    move |server, request| {
        let request = serde_json::from_str::<Req>(request)
            .map_err(|error| RemoteFnError::DeserializeRequest(error, request.into()));
        match request {
            Ok(request) => UpliftFuture::Future(function(server, request)),
            Err(error) => UpliftFuture::DeserializeRequest(error),
        }
    }
}

#[pin_project(project = UpliftFutureProj)]
pub enum UpliftFuture<F> {
    DeserializeRequest(RemoteFnError),
    Future(#[pin] F),
}

impl<F, Res, E> Future for UpliftFuture<F>
where
    F: Future<Output = Result<Res, E>>,
    Res: serde::Serialize,
    Status: From<E>,
{
    type Output = Result<String, RemoteFnError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            UpliftFutureProj::DeserializeRequest(error) => {
                let error = std::mem::replace(error, RemoteFnError::RemoteFnsNotSet);
                Err(error)
            }
            UpliftFutureProj::Future(future) => match ready!(future.poll(cx)) {
                Ok(response) => {
                    serde_json::to_string(&response).map_err(RemoteFnError::SerializeResponse)
                }
                Err(error) => Err(RemoteFnError::ServerFn(Box::new(error.into()))),
            },
        }
        .into()
    }
}
