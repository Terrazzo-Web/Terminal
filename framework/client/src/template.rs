use crate::debug_correlation_id::DebugCorrelationId;
use crate::signal::depth::Depth;

pub trait IsTemplate: Clone + 'static {
    type Value;
    fn apply<R: Into<Self::Value>>(self, new: impl FnOnce() -> R);
    fn depth(&self) -> Depth;
    fn debug_id(&self) -> &DebugCorrelationId<impl std::fmt::Display>;
}

pub trait IsTemplated {
    type Template: IsTemplate<Value = Self>;
}
