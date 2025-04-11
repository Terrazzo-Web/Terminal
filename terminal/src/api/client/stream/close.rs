use super::dispatcher::DISPATCHERS;
use crate::api::TerminalAddress;

pub async fn close(terminal: &TerminalAddress) {
    let mut dispatchers = DISPATCHERS.lock();
    let Some(dispatchers) = &mut *dispatchers else {
        return;
    };
    let Some(mut dispatcher) = dispatchers.download.remove(&terminal.id) else {
        return;
    };
    dispatcher.close_channel();
}
