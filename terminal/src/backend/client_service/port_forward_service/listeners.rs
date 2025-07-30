#![cfg(feature = "server")]

use std::collections::HashMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use tokio::net::TcpStream;
use tokio::sync::mpsc;

#[must_use]
pub fn listeners() -> MutexGuard<'static, Listeners> {
    static LISTENERS: Mutex<Listeners> = Mutex::new(Listeners::new());
    LISTENERS.lock().expect("listeners")
}

pub struct Listeners(Option<ListenersMap>);

type ListenersMap = HashMap<EndpointId, mpsc::Receiver<TcpStream>>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EndpointId {
    pub host: String,
    pub port: i32,
}

impl Listeners {
    const fn new() -> Self {
        Self(None)
    }
}

impl Deref for Listeners {
    type Target = ListenersMap;

    fn deref(&self) -> &Self::Target {
        static DEFAULT: OnceLock<ListenersMap> = OnceLock::new();
        self.0
            .as_ref()
            .unwrap_or_else(|| DEFAULT.get_or_init(|| ListenersMap::new()))
    }
}

impl DerefMut for Listeners {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_or_insert(HashMap::default())
    }
}
