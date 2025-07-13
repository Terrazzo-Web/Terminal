#![cfg(feature = "server")]

use nameth::nameth;
use tonic::Status;

use crate::backend::client_service::remote_fn_service;
use crate::converter::api::GetConversionsRequest;
use crate::converter::api::GetConversionsResponse;

#[nameth]
pub async fn get_conversions(content: String) -> Result<Vec<GetConversionsResponse>, Status> {
    Ok(vec![GetConversionsResponse {
        language: "todo!()".into(),
        conversio: content,
    }])
}

remote_fn_service::declare_remote_fn!(
    GET_CONVERSIONS_FN,
    GET_CONVERSIONS,
    GetConversionsRequest,
    Vec<GetConversionsResponse>,
    |_server, arg| get_conversions(arg.content)
);
