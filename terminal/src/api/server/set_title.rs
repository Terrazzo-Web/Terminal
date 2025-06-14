use std::sync::Arc;

use terrazzo::axum::Json;
use trz_gateway_common::http_error::HttpError;
use trz_gateway_server::server::Server;

use crate::api::SetTitleRequest;
use crate::backend::client_service::set_title;
use crate::backend::client_service::set_title::SetTitleError;
use crate::backend::protos::terrazzo::gateway::client::MaybeString;
use crate::backend::protos::terrazzo::gateway::client::SetTitleRequest as SetTitleRequestProto;

pub async fn set_title(
    server: Arc<Server>,
    Json(request): Json<SetTitleRequest>,
) -> Result<(), HttpError<SetTitleError>> {
    let client_address = request.terminal.via.to_vec();
    Ok(set_title::set_title(
        &server,
        &client_address,
        SetTitleRequestProto {
            address: Some(request.terminal.into()),
            shell_title: request.title.shell_title,
            override_title: request.title.override_title.map(|s| MaybeString { s }),
        },
    )
    .await?)
}
