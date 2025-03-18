use std::sync::Arc;

use terrazzo::autoclone;
use terrazzo::axum::Router;
use terrazzo::axum::response::IntoResponse as _;
use terrazzo::axum::response::Response;
use terrazzo::axum::routing::get;
use terrazzo::axum::routing::post;
use terrazzo::http::HeaderMap;
use terrazzo::http::HeaderName;
use terrazzo::http::StatusCode;
use trz_gateway_server::server::Server;

mod correlation_id;
mod list;
mod new_id;
mod remotes;
mod resize;
mod set_order;
mod set_title;
mod stream;
mod write;

const ERROR_HEADER: HeaderName = HeaderName::from_static(super::ERROR_HEADER);

#[autoclone]
pub fn route(server: Arc<Server>) -> Router {
    Router::new()
        .route("/list", get(list::list))
        .route("/new_id", post(new_id::new_id))
        .route("/stream/pipe", post(stream::pipe))
        .route("/stream/pipe/close", post(stream::close_pipe))
        .route(
            "/stream/register",
            post(|request| {
                autoclone!(server);
                stream::register(server, request)
            }),
        )
        .route("/stream/close/{terminal_id}", post(stream::close))
        .route("/resize/{terminal_id}", post(resize::resize))
        .route("/set_title/{terminal_id}", post(set_title::set_title))
        .route("/set_order", post(set_order::set_order))
        .route("/write/{terminal_id}", post(write::write))
        .route(
            "/remotes",
            get(|request| {
                autoclone!(server);
                remotes::remotes(server)
            }),
        )
}

fn into_error<E: std::error::Error>(status_code: StatusCode) -> impl FnMut(E) -> Response {
    move |error| {
        if let Ok(error_header) = error.to_string().parse() {
            let mut headers = HeaderMap::new();
            headers.insert(ERROR_HEADER, error_header);
            (status_code, headers).into_response()
        } else {
            (status_code, error.to_string()).into_response()
        }
    }
}
