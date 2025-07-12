use tonic::body::Body as BoxBody;
use tonic::client::GrpcService;
use tonic::codegen::Bytes;
use tonic::codegen::StdError;
use tonic::transport::Body;
use trz_gateway_server::server::Server;

use crate::backend::client_service::remote_fn_service::REMOTE_FNS;
use crate::backend::client_service::remote_fn_service::RemoteFnError;
use crate::backend::client_service::routing::DistributedCallback;
use crate::backend::protos::terrazzo::remotefn::RemoteFnRequest;
use crate::backend::protos::terrazzo::remotefn::remote_fn_service_client::RemoteFnServiceClient;
use crate::backend::protos::terrazzo::shared::ClientAddress as ClientAddressProto;

pub struct DistributedFn;

impl DistributedCallback for DistributedFn {
    type Request = RemoteFnRequest;
    type Response = String;
    type LocalError = RemoteFnError;
    type RemoteError = tonic::Status;

    async fn local(server: &Server, request: RemoteFnRequest) -> Result<String, RemoteFnError> {
        let Some(remote_server_fns) = REMOTE_FNS.get() else {
            return Err(RemoteFnError::RemoteFnsNotSet);
        };
        let Some(remote_server_fn) = remote_server_fns.get(request.server_fn_name.as_str()) else {
            return Err(RemoteFnError::RemoteFnNotFound(request.server_fn_name));
        };
        let callback = &remote_server_fn.callback;
        return callback(server, &request.json).await;
    }

    async fn remote<T>(
        channel: T,
        client_address: &[impl AsRef<str>],
        mut request: RemoteFnRequest,
    ) -> Result<String, tonic::Status>
    where
        T: GrpcService<BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        request.address = Some(ClientAddressProto::of(client_address));
        let result = RemoteFnServiceClient::new(channel)
            .call_server_fn(request)
            .await?
            .into_inner();
        Ok(result.json)
    }
}
