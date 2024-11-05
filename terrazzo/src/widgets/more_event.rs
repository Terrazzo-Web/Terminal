use named::named;
use named::NamedEnumValues as _;
use terrazzo_client::prelude::XString;
use wasm_bindgen::JsCast;
use web_sys::EventTarget;

pub trait MoreEvent {
    fn current_target_element<T: JsCast>(
        &self,
        name: impl Into<XString>,
    ) -> Result<T, CurrentTargetElementError>;
}

impl MoreEvent for web_sys::Event {
    fn current_target_element<T: JsCast>(
        &self,
        name: impl Into<XString>,
    ) -> Result<T, CurrentTargetElementError> {
        let Some(current_target) = self.current_target() else {
            return Err(CurrentTargetElementError::Missing(name.into()));
        };
        current_target
            .dyn_into()
            .map_err(|wrong_type| CurrentTargetElementError::WrongType(name.into(), wrong_type))
    }
}

#[named]
#[derive(thiserror::Error, Debug)]
pub enum CurrentTargetElementError {
    #[error("[{}] [{0}] Missing current target", self.name())]
    Missing(XString),

    #[error("[{}] [{0}] Wrong type: {1:?}", self.name())]
    WrongType(XString, EventTarget),
}
