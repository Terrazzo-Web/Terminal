use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::SeqCst;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConsumerId(i32);

impl ConsumerId {
    pub fn new() -> Self {
        static NEXT: AtomicI32 = AtomicI32::new(1);
        Self(NEXT.fetch_add(1, SeqCst))
    }
}

impl std::fmt::Display for ConsumerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}
