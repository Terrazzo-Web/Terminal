#[cfg(not(feature = "concise_traces"))]
pub type DebugCorrelationId<N> = with_debug::DebugCorrelationId<N>;

#[cfg(feature = "concise_traces")]
pub type DebugCorrelationId<N> = without_debug::DebugCorrelationId<N>;

#[cfg(not(feature = "concise_traces"))]
mod with_debug {
    use named::named;
    use named::NamedType as _;
    use tracing::trace;

    #[named]
    pub struct DebugCorrelationId<N: std::fmt::Display>(N, i32);

    impl<N: std::fmt::Display> std::fmt::Debug for DebugCorrelationId<N> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_tuple(Self::type_name())
                .field(&self.0.to_string())
                .field(&self.1)
                .finish()
        }
    }

    impl<N: std::fmt::Display> std::fmt::Display for DebugCorrelationId<N> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}:{:#x}", self.0, self.1)
        }
    }

    impl<N: std::fmt::Display> DebugCorrelationId<N> {
        pub fn new(name: impl FnOnce() -> N) -> Self {
            use std::sync::atomic::AtomicI32;
            use std::sync::atomic::Ordering::SeqCst;
            static NEXT: AtomicI32 = AtomicI32::new(0);
            let this = Self(name(), NEXT.fetch_add(1, SeqCst));
            trace!(debug_correlation_id = %this, "New");
            return this;
        }
    }

    impl<N: std::fmt::Display> Drop for DebugCorrelationId<N> {
        fn drop(&mut self) {
            trace!(debug_correlation_id = %self, "Drop");
        }
    }
}

#[allow(unused)]
mod without_debug {
    use std::fmt::format;
    use std::marker::PhantomData;

    use named::named;
    use named::NamedType as _;

    #[derive(Debug)]
    #[named]
    pub struct DebugCorrelationId<N: std::fmt::Display>(PhantomData<N>);

    impl<N: std::fmt::Display> std::fmt::Display for DebugCorrelationId<N> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Display::fmt(Self::type_name(), f)
        }
    }

    impl<N: std::fmt::Display> DebugCorrelationId<N> {
        pub fn new(_: impl FnOnce() -> N) -> Self {
            DebugCorrelationId(PhantomData)
        }
    }
}
