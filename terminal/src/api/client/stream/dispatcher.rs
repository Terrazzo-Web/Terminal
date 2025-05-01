use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::MutexGuard;

use futures::channel::mpsc;

use crate::api::WriteRequest;
use crate::terminal_id::TerminalId;

pub static DISPATCHERS: Dispatchers = Dispatchers(Mutex::new(None));

pub(super) struct Dispatchers(std::sync::Mutex<Option<DispatchersInner>>);

pub struct DispatchersInner {
    pub(super) download: HashMap<TerminalId, mpsc::Sender<Vec<u8>>>,
    pub(super) upload: mpsc::Sender<WriteRequest>,
}

impl Dispatchers {
    pub(super) fn lock(&self) -> MutexGuard<'_, Option<DispatchersInner>> {
        self.0.lock().unwrap()
    }
}
