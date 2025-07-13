use server_fn::Http;
use server_fn::ServerFnError;
use server_fn::codec::Json;
use terrazzo::server;

use crate::api::client_address::ClientAddress;

#[server(protocol = Http<Json, Json>)]
pub async fn get_conversions(
    remote: Option<ClientAddress>,
    content: String,
) -> Result<Vec<GetConversionsResponse>, ServerFnError> {
    Ok(super::service::GET_CONVERSIONS_FN
        .call(
            remote.unwrap_or_default(),
            GetConversionsRequest { content },
        )
        .await?)
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct GetConversionsRequest {
    pub content: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct GetConversionsResponse {
    pub language: String,
    pub conversion: String,
}
