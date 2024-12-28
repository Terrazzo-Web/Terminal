use std::rc::Rc;
use std::sync::Mutex;

use tracing::trace;
use web_sys::Element;

use crate::debug_correlation_id::DebugCorrelationId;
use crate::element::XElement;
use crate::element::XElementValue;
use crate::key::XKey;
use crate::node::XNode;
use crate::signal::depth::Depth;

#[derive(Clone)]
pub struct XTemplate(Rc<XTemplateInner>);

struct XTemplateInner {
    debug_id: DebugCorrelationId<&'static str>,
    depth: Depth,
    element_mut: Rc<Mutex<Element>>,
    old: Mutex<Option<XElement>>,
}

impl XTemplate {
    pub fn new(element_mut: Rc<Mutex<Element>>) -> Self {
        Self::with_depth(Depth::zero(), element_mut)
    }

    pub(crate) fn with_depth(depth: Depth, element_mut: Rc<Mutex<Element>>) -> Self {
        Self(Rc::new(XTemplateInner {
            debug_id: DebugCorrelationId::new(|| "template"),
            depth,
            element_mut,
            old: Mutex::default(),
        }))
    }

    pub(crate) fn depth(&self) -> Depth {
        self.0.depth
    }

    pub fn apply(self, new: impl FnOnce() -> XElement) {
        let mut new = new();
        reindex_nodes(&mut new);
        {
            let mut old = self.0.old.lock().unwrap();
            if let Some(old) = &mut *old {
                new.merge(self.depth(), old, self.0.element_mut.clone())
            } else {
                let mut old = new.zero();
                new.merge(self.depth(), &mut old, self.0.element_mut.clone())
            };
            *old = Some(new);
        }
        trace! { "The template is updated to {:?}", self.0.old.lock().unwrap() };
    }

    pub fn element(&self) -> Element {
        self.0.element_mut.lock().expect("element").clone()
    }

    #[cfg(not(feature = "concise_traces"))]
    pub(crate) fn with_old(&self, f: impl FnOnce(&Option<XElement>)) {
        f(&self.0.old.lock().expect("old"))
    }

    pub(crate) fn debug_id(&self) -> &DebugCorrelationId<impl std::fmt::Display> {
        &self.0.debug_id
    }
}

fn reindex_nodes(new: &mut XElement) {
    let XElementValue::Static { children, .. } = &mut new.value else {
        return;
    };
    let mut next = 0;
    for child in children {
        let XNode::Element(child) = child else {
            continue;
        };
        reindex_nodes(child);
        let XKey::Index(index) = &mut child.key else {
            continue;
        };
        *index = next;
        next += 1;
    }
}
