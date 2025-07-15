#![cfg(feature = "server")]

use nameth::nameth;
use tonic::Status;

use super::api::Conversion;
use super::api::Conversions;
use super::api::ConversionsRequest;
use crate::backend::client_service::remote_fn_service;
use crate::converter::api::Language;

#[nameth]
pub async fn get_conversions(input: String) -> Result<Conversions, Status> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&input) {
        if let Ok(json) = serde_json::to_string_pretty(&json) {
            return Ok(Conversions {
                conversions: vec![
                    Conversion::new(Language::new("json"), json.clone()),
                    Conversion::new(Language::new("json2"), json),
                ]
                .into(),
            });
        }
    }
    return Err(Status::invalid_argument("Not valid json"));
}

remote_fn_service::declare_remote_fn!(
    GET_CONVERSIONS_FN,
    GET_CONVERSIONS,
    ConversionsRequest,
    Conversions,
    |_server, arg| get_conversions(arg.input)
);
