#![cfg(feature = "server")]

use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

use crate::backend::client_service::remote_fn;

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCheckRequest {
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "b"))]
    pub base_path: Arc<str>,
    #[cfg_attr(not(feature = "diagnostics"), serde(rename = "f"))]
    pub features: Vec<String>,
}

remote_fn::declare_remote_fn!(
    CARGO_CHECK_REMOTE_FN,
    super::CARGO_CHECK,
    |_server, arg: CargoCheckRequest| {
        async move {
            super::service::cargo_check(
                arg.base_path.as_ref(),
                &arg.features.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            )
            .await
        }
    }
);
