use std::sync::Arc;
use std::sync::Mutex;

use scopeguard::defer;
use tracing::debug;
use tracing::debug_span;
use tracing::error;
use tracing::trace;

use self::batch::Batch;
use self::batch::NotBatched;
use self::depth::Depth;
use self::producers::producer::ProducedValue;
use self::producers::producer::Producer;
use self::version::Version;
use super::string::XString;
use crate::debug_correlation_id::DebugCorrelationId;
use crate::prelude::Consumers;

pub mod batch;
pub mod depth;
pub mod derive;
pub mod mutable_signal;
mod producers;
pub mod reactive_closure;
mod tests;
mod version;
mod weak;

use self::inner::XSignalInner;

/// A mutable value that callbacks can subscribe to.
///
/// - Derived signals
/// - ReactiveClosures re-compute and update HTML nodes when signals change
pub struct XSignal<T>(Arc<XSignalInner<T>>);

mod inner {
    use std::ops::Deref;
    use std::sync::Arc;
    use std::sync::Mutex;

    use super::producers::producer::Producer;
    use super::ProducedSignal;
    use super::XSignal;
    use super::XSignalValue;

    pub struct XSignalInner<T> {
        pub(super) current_value: Mutex<XSignalValue<T>>,
        pub(super) producer: Producer<ProducedSignal>,
        pub(super) immutable_value: Arc<Mutex<Option<T>>>,
        pub(super) on_drop: Mutex<Vec<Box<dyn FnOnce()>>>,
    }

    impl<T> Deref for XSignal<T> {
        type Target = XSignalInner<T>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

pub struct ProducedSignal;

impl ProducedValue for ProducedSignal {
    /// The SortKey is [Depth], ensuring that parent nodes are recomputed before child nodes.
    type SortKey = Depth;
    type Value = Version;
}

#[derive(Clone, Debug)]
struct XSignalValue<T> {
    value: Option<T>,
    version: Version,
}

impl<T> XSignalValue<T> {
    fn value(&self) -> &T {
        let Some(value) = &self.value else {
            error!("Value should never be null until dropped");
            panic!()
        };
        value
    }
}

impl<T> XSignal<T> {
    pub fn new(name: impl Into<XString>, value: T) -> Self {
        Self(Arc::new(XSignalInner {
            current_value: Mutex::new(XSignalValue {
                value: Some(value),
                version: Version::current(),
            }),
            producer: Producer::new(name.into()),
            immutable_value: Arc::default(),
            on_drop: Mutex::new(vec![]),
        }))
    }

    #[must_use]
    pub fn add_subscriber(&self, closure: impl Fn(T) + 'static) -> Consumers
    where
        T: Clone + 'static,
    {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering::SeqCst;
        let weak = self.downgrade();
        let last_version = AtomicUsize::new(0);
        let closure = move |version: Version| {
            let Some(this) = weak.upgrade() else { return };
            let version = version.number();
            let last_version = last_version.swap(version, SeqCst);
            if last_version < version {
                defer!(trace!("End"));
                trace!(last_version, version, "Start");
                closure(this.get_value_untracked())
            } else {
                debug!(last_version, version, "Skip");
            }
        };
        Consumers(vec![self.producer.register(
            DebugCorrelationId::new(|| "[closure]".into()),
            Depth::zero(),
            closure,
        )])
    }

    pub fn get_value_untracked(&self) -> T
    where
        T: Clone,
    {
        self.current_value.lock().unwrap().value().clone()
    }
}

impl<T> Clone for XSignal<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: std::fmt::Debug + 'static> XSignal<T> {
    pub fn set(&self, new_value: impl Into<T>)
    where
        T: Eq,
    {
        let _span = debug_span!("Set", signal = %self.producer.name()).entered();
        self.update_impl(|old_value| {
            let new_value = new_value.into();
            (new_value != *old_value).then_some(new_value)
        });
    }

    pub fn update<R, U>(&self, compute: impl FnOnce(&T) -> U) -> R
    where
        U: Into<UpdateSignalResult<Option<T>, R>>,
    {
        let _span = debug_span!("Update", signal = %self.producer.name()).entered();
        self.update_impl(compute)
    }

    fn update_impl<R, U>(&self, compute: impl FnOnce(&T) -> U) -> R
    where
        U: Into<UpdateSignalResult<Option<T>, R>>,
    {
        let (version, result) = {
            let mut current = self.current_value.lock().expect("current");
            let current_value = current.value();
            let current_version = current.version.number();
            let UpdateSignalResult { new_value, result } = compute(current_value).into();
            let Some(new_value) = new_value else {
                debug! { "Signal value is not changing version:{current_version} value:{current_value:?}" };
                return result;
            };
            let new_version: Version = Version::next();
            debug!(
                "Signal value has changed old@{}={current_value:?} vs new@{}={new_value:?}",
                current_version,
                new_version.number()
            );
            current.value = Some(new_value);
            current.version = new_version;
            (new_version, result)
        };
        self.process_or_batch(version);
        return result;
    }

    pub fn force(&self, new_value: impl Into<T>) {
        let _span = debug_span!("Force", signal = %self.producer.name()).entered();
        let new_value = new_value.into();
        let version = {
            let mut current = self.current_value.lock().expect("current");
            current.value = Some(new_value);
            current.version = Version::next();
            debug! { "Signal value was forced to version:{} value:{:?}", current.version.number(), current.value() };
            current.version
        };
        self.process_or_batch(version);
    }

    fn process_or_batch(&self, version: Version) {
        Batch::try_push(|| {
            let this = self.to_owned();
            trace!("Update is batched");
            move |version| this.process(version)
        })
        .unwrap_or_else(|NotBatched { .. }| {
            trace!("Update is applied immediately");
            self.process(version)
        });
    }

    fn process(&self, version: Version) {
        self.producer.process(version);
    }
}

pub struct UpdateSignalResult<T, R> {
    pub new_value: T,
    pub result: R,
}

impl<T> From<T> for UpdateSignalResult<T, ()> {
    fn from(new_value: T) -> Self {
        Self {
            new_value,
            result: (),
        }
    }
}

pub trait UpdateAndReturn {
    type NewValue;
    fn and_return<R>(self, result: R) -> UpdateSignalResult<Self::NewValue, R>;
}

impl<T> UpdateAndReturn for T {
    type NewValue = T;

    fn and_return<R>(self, result: R) -> UpdateSignalResult<Self::NewValue, R> {
        UpdateSignalResult {
            new_value: self,
            result,
        }
    }
}

impl<T> Drop for XSignalInner<T> {
    fn drop(&mut self) {
        debug!(signal = %self.producer.name(), "Dropped");
        if Arc::strong_count(&self.immutable_value) > 1 {
            let mut immutable_value = self.immutable_value.lock().unwrap();
            *immutable_value = self.current_value.lock().unwrap().value.take();
        }
        let mut on_drop = self.on_drop.lock().unwrap();
        for on_drop in std::mem::take(&mut *on_drop) {
            on_drop()
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for XSignal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("XSignal")
            .field(self.current_value.lock().unwrap().value())
            .finish()
    }
}
