#![cfg(feature = "server")]

use nameth::nameth;
use tonic::Status;

use crate::backend::client_service::remote_fn_service;
use crate::converter::api::GetConversionsRequest;
use crate::converter::api::GetConversionsResponse;

#[nameth]
pub async fn get_conversions(content: String) -> Result<Vec<GetConversionsResponse>, Status> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Ok(json) = serde_json::to_string_pretty(&json) {
            return Ok(vec![GetConversionsResponse {
                language: "json".into(),
                conversion: json,
            }]);
        }
    }
    return Err(Status::invalid_argument("Not valid json"));
}

remote_fn_service::declare_remote_fn!(
    GET_CONVERSIONS_FN,
    GET_CONVERSIONS,
    GetConversionsRequest,
    Vec<GetConversionsResponse>,
    |_server, arg| get_conversions(arg.content)
);
