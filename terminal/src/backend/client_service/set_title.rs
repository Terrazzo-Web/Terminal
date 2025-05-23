use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use tonic::Status;
use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use tracing::Instrument as _;
use tracing::debug;
use tracing::debug_span;
use trz_gateway_common::http_error::IsHttpError;
use trz_gateway_server::server::Server;

use super::routing::DistributedCallback;
use super::routing::DistributedCallbackError;
use crate::api::TabTitle;
use crate::backend::protos::terrazzo::gateway::client::ClientAddress;
use crate::backend::protos::terrazzo::gateway::client::Empty;
use crate::backend::protos::terrazzo::gateway::client::SetTitleRequest;
use crate::backend::protos::terrazzo::gateway::client::client_service_client::ClientServiceClient;
use crate::processes;
use crate::processes::set_title::SetTitleError as SetTitleErrorImpl;

pub fn set_title(
    server: &Server,
    client_address: &[impl AsRef<str>],
    request: SetTitleRequest,
) -> impl Future<Output = Result<(), SetTitleError>> {
    async {
        debug!("Start");
        defer!(debug!("Done"));
        Ok(SetTitleCallback::process(server, client_address, request).await?)
    }
    .instrument(debug_span!("SetTitle"))
}

struct SetTitleCallback;

impl DistributedCallback for SetTitleCallback {
    type Request = SetTitleRequest;
    type Response = ();
    type LocalError = SetTitleErrorImpl;
    type RemoteError = tonic::Status;

    async fn local(_: &Server, request: SetTitleRequest) -> Result<(), SetTitleErrorImpl> {
        let terminal_id = request.address.unwrap_or_default().terminal_id.into();
        processes::set_title::set_title(
            &terminal_id,
            TabTitle {
                shell_title: request.shell_title,
                override_title: request.override_title.map(|s| s.s),
            },
        )
    }

    async fn remote<T>(
        mut client: ClientServiceClient<T>,
        client_address: &[impl AsRef<str>],
        mut request: SetTitleRequest,
    ) -> Result<(), tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let Empty {} = client.set_title(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] {0}", n = self.name())]
    SetTitleError(#[from] DistributedCallbackError<SetTitleErrorImpl, tonic::Status>),
}

impl IsHttpError for SetTitleError {
    fn status_code(&self) -> terrazzo::http::StatusCode {
        match self {
            Self::SetTitleError(error) => error.status_code(),
        }
    }
}

impl From<SetTitleError> for Status {
    fn from(error: SetTitleError) -> Self {
        match error {
            SetTitleError::SetTitleError(error) => error.into(),
        }
    }
}

impl From<SetTitleErrorImpl> for Status {
    fn from(error: SetTitleErrorImpl) -> Self {
        match error {
            error @ SetTitleErrorImpl::TerminalNotFound { .. } => {
                Status::not_found(error.to_string())
            }
        }
    }
}
