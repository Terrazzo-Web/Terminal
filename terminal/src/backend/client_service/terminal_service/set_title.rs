use std::sync::Arc;

use nameth::NamedEnumValues as _;
use nameth::nameth;
use scopeguard::defer;
use terrazzo::http::StatusCode;
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

use crate::api::shared::terminal_schema::TabTitle;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::client_service::routing::DistributedCallbackError;
use crate::backend::protos::terrazzo::shared::ClientAddress;
use crate::backend::protos::terrazzo::shared::Empty;
use crate::backend::protos::terrazzo::terminal::SetTitleRequest;
use crate::backend::protos::terrazzo::terminal::terminal_service_client::TerminalServiceClient;
use crate::processes;
use crate::processes::set_title::SetTitleError as SetTitleErrorImpl;

pub fn set_title(
    server: &Arc<Server>,
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
    type RemoteError = Status;

    async fn local(_: &Arc<Server>, request: SetTitleRequest) -> Result<(), SetTitleErrorImpl> {
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
        channel: T,
        client_address: &[impl AsRef<str>],
        mut request: SetTitleRequest,
    ) -> Result<(), Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address.get_or_insert_default().via = Some(ClientAddress::of(client_address));
        let mut client = TerminalServiceClient::new(channel);
        let Empty {} = client.set_title(request).await?.into_inner();
        Ok(())
    }
}

#[nameth]
#[derive(thiserror::Error, Debug)]
pub enum SetTitleError {
    #[error("[{n}] {0}", n = self.name())]
    SetTitleError(#[from] DistributedCallbackError<SetTitleErrorImpl, Status>),
}

impl IsHttpError for SetTitleError {
    fn status_code(&self) -> StatusCode {
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
