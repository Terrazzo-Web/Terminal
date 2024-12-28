use crate::debug_correlation_id::DebugCorrelationId;
use crate::signal::depth::Depth;

pub trait IsTemplate: Clone + 'static {
    type Value;
    fn apply(self, new: impl FnOnce() -> Self::Value);
    fn depth(&self) -> Depth;
    fn debug_id(&self) -> &DebugCorrelationId<impl std::fmt::Display>;
}

pub trait IsTemplated {
    type Template: IsTemplate<Value = Self>;
}
