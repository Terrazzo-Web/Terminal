use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

use scopeguard::defer;
use tracing::debug;
use tracing::trace;
use tracing::trace_span;

use super::version::Version;
use crate::debug_correlation_id::DebugCorrelationId;
use crate::string::XString;

pub mod reactive_closure_builder;

/// A closure that depends on [XSignal]s and that gets recomputed when signal values change.
struct ReactiveClosure<F> {
    /// A name, for troubleshooting purposes only.
    name: DebugCorrelationId<XString>,

    /// The closure
    reactive_closure: F,

    /// The version that was active the last time the closure was called.
    /// The closure is called only if versions change.
    last_version: AtomicUsize,
}

impl<F: Fn()> ReactiveClosure<F> {
    fn call(&self, version: Version) {
        let version = version.number();
        let last_version = self.last_version.swap(version, SeqCst);
        let _span = trace_span!("Call", closure = %self.name).entered();
        if last_version < version {
            defer!(trace!("End"));
            trace!(last_version, version, "Start");
            (self.reactive_closure)();
        } else {
            debug!(last_version, version, "Skip");
        }
    }
}

impl<F> Drop for ReactiveClosure<F> {
    fn drop(&mut self) {
        trace!(closure = %self.name, "ReactiveClosure dropped");
    }
}
