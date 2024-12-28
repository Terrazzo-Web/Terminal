use debug_correlation_id::DebugCorrelationId;
use named::NamedType as _;

mod attribute;
mod debug_correlation_id;
mod element;
mod key;
mod node;
pub mod owned_closure;
pub mod prelude;
mod signal;
mod string;
mod template;
mod utils;

pub fn setup_logging() {
    use tracing_subscriber_wasm::MakeConsoleWriter;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(MakeConsoleWriter::default())
        .without_time()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .init();
    let version = "1.0";
    tracing::trace!(version, "Setting logging: TRACE");
    tracing::debug!(version, "Setting logging: DEBUG");
    tracing::info!(version, "Setting logging: INFO");
    tracing::info!(
        "{}: {:?}",
        DebugCorrelationId::<&str>::type_name(),
        DebugCorrelationId::new(|| "here")
    );
}
