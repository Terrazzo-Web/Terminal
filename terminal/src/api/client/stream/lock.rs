use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::MutexGuard;

use super::DispatchersInner;

pub struct DispatchersLock<'t>(MutexGuard<'t, Option<DispatchersInner>>);

impl<'t> DispatchersLock<'t> {
    pub fn new(lock: MutexGuard<'t, Option<DispatchersInner>>) -> Self {
        assert!(lock.is_some());
        Self(lock)
    }
}

impl Deref for DispatchersLock<'_> {
    type Target = DispatchersInner;

    fn deref(&self) -> &Self::Target {
        let inner = self.0.deref().as_ref();
        unsafe { inner.unwrap_unchecked() }
    }
}

impl DerefMut for DispatchersLock<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let inner = self.0.deref_mut().as_mut();
        unsafe { inner.unwrap_unchecked() }
    }
}
