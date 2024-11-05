use super::XSignal;

pub struct MutableSignal<V> {
    signal: XSignal<V>,
}

impl<V> From<XSignal<V>> for MutableSignal<V> {
    fn from(signal: XSignal<V>) -> Self {
        Self { signal }
    }
}

impl<V> From<&XSignal<V>> for MutableSignal<V> {
    fn from(signal: &XSignal<V>) -> Self {
        Self {
            signal: signal.clone(),
        }
    }
}

impl<T: std::fmt::Debug + 'static> MutableSignal<T> {
    pub fn set(&self, new_value: impl Into<T>)
    where
        T: Eq,
    {
        self.signal.set(new_value)
    }

    pub fn update(&self, compute: impl FnOnce(&T) -> Option<T>) {
        self.signal.update(compute);
    }

    pub fn force(&self, new_value: impl Into<T>) {
        self.signal.force(new_value);
    }
}

impl<V> Clone for MutableSignal<V> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
        }
    }
}
