use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

/// Incremental numbers representing the version of the state.
/// Reactive callbacks subscribed to signals are re-computed when signals change,
/// but only once per version change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(usize);

impl Version {
    pub(super) fn next() -> Version {
        Self(NEXT_VERSION.fetch_add(1, SeqCst))
    }

    pub fn current() -> Version {
        Self(NEXT_VERSION.load(SeqCst) - 1)
    }

    pub fn number(self) -> usize {
        self.0
    }
}

/// Incremental version numbers.
/// Reactive callbacks start with last_version=0 before their first call.
/// New signals must then start with version>0 to ensure the first call.
/// If the very first version number is 1, the starting 'NEXT_VERSION' value must be 2.
static NEXT_VERSION: AtomicUsize = AtomicUsize::new(2);

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn version() {
        let next = Version::next();
        assert_eq!(next, Version::current());
        assert_eq!(next, Version::current());
        assert_eq!(next.0 + 1, Version::next().0)
    }
}
