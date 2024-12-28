use std::rc::Rc;

use named::named;
use named::NamedType as _;
use tracing::trace;
use tracing::warn;
use web_sys::Element;

use self::inner::AttributeTemplateInner;
use crate::debug_correlation_id::DebugCorrelationId;
use crate::signal::depth::Depth;
use crate::signal::reactive_closure::reactive_closure_builder::Consumers;
use crate::string::XString;
use crate::template::IsTemplate;
use crate::template::IsTemplated;

#[named]
pub struct XAttribute {
    pub name: XString,
    pub value: XAttributeValue,
}

pub enum XAttributeValue {
    Static(XString),
    Dynamic(XDynamicAttribute),
    Generated {
        template: XAttributeTemplate,
        consumers: Consumers,
    },
}

pub struct XDynamicAttribute(pub Box<dyn Fn(XAttributeTemplate) -> Consumers>);

impl<F: Fn(XAttributeTemplate) -> Consumers + 'static> From<F> for XDynamicAttribute {
    fn from(value: F) -> Self {
        Self(Box::new(value))
    }
}

#[derive(Clone)]
pub struct XAttributeTemplate(Rc<AttributeTemplateInner>);

mod inner {
    use std::ops::Deref;

    use web_sys::Element;

    use super::XAttributeTemplate;
    use crate::debug_correlation_id::DebugCorrelationId;
    use crate::signal::depth::Depth;
    use crate::string::XString;

    pub struct AttributeTemplateInner {
        pub element: Element,
        pub attribute_name: XString,
        pub(super) debug_id: DebugCorrelationId<String>,
        pub(super) depth: Depth,
    }

    impl Deref for XAttributeTemplate {
        type Target = AttributeTemplateInner;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}

impl IsTemplate for XAttributeTemplate {
    type Value = XAttributeValue;

    fn apply(self, new: impl FnOnce() -> Self::Value) {
        let mut new = XAttribute {
            name: self.attribute_name.clone(),
            value: new(),
        };
        new.merge(self.depth, None, &self.element);
    }

    fn depth(&self) -> Depth {
        self.depth
    }

    fn debug_id(&self) -> &DebugCorrelationId<impl std::fmt::Display> {
        &self.debug_id
    }
}

impl IsTemplated for XAttributeValue {
    type Template = XAttributeTemplate;
}

impl<T> From<T> for XAttributeValue
where
    XString: From<T>,
{
    fn from(value: T) -> Self {
        Self::Static(value.into())
    }
}

impl XAttribute {
    pub fn merge(
        &mut self,
        depth: Depth,
        old_attribute_value: Option<XAttributeValue>,
        element: &Element,
    ) {
        let new_attribute = self;
        let attribute_name = &new_attribute.name;
        match &new_attribute.value {
            XAttributeValue::Static(new_attribute_value) => {
                merge_static_atttribute(
                    element,
                    attribute_name,
                    new_attribute_value,
                    old_attribute_value,
                );
            }
            XAttributeValue::Dynamic(XDynamicAttribute(new_attribute_value)) => {
                new_attribute.value = merge_dynamic_atttribute(
                    depth,
                    element,
                    attribute_name,
                    new_attribute_value,
                    old_attribute_value,
                );
            }
            XAttributeValue::Generated { .. } => {
                warn!("Illegal {} state", XAttribute::type_name());
                debug_assert!(false);
            }
        }
    }
}

fn merge_static_atttribute(
    element: &Element,
    attribute_name: &XString,
    new_attribute_value: &XString,
    old_attribute_value: Option<XAttributeValue>,
) {
    if let Some(XAttributeValue::Static(old_attribute_value)) = &old_attribute_value {
        if new_attribute_value == old_attribute_value {
            trace!("Attribute '{attribute_name}' is still '{new_attribute_value}'");
            return;
        }
    }
    drop(old_attribute_value);
    match element.set_attribute(attribute_name, new_attribute_value) {
        Ok(()) => {
            trace! { "Set attribute '{attribute_name}' to '{new_attribute_value}'" };
        }
        Err(error) => {
            warn! { "Set attribute '{attribute_name}' to '{new_attribute_value}' failed: {error:?}" };
        }
    }
}

fn merge_dynamic_atttribute(
    depth: Depth,
    element: &Element,
    attribute_name: &XString,
    new_attribute_value: &dyn Fn(XAttributeTemplate) -> Consumers,
    old_attribute_value: Option<XAttributeValue>,
) -> XAttributeValue {
    let new_template = if let Some(XAttributeValue::Generated {
        template: old_template,
        consumers: old_consumers,
    }) = old_attribute_value
    {
        trace!("Reuse exising attribute template {attribute_name}");
        drop(old_consumers);
        old_template
    } else {
        trace!("Create a new attribute template {attribute_name}");
        XAttributeTemplate(Rc::new(AttributeTemplateInner {
            element: element.clone(),
            attribute_name: attribute_name.clone(),
            debug_id: DebugCorrelationId::new(|| format!("attribute_template:{attribute_name}")),
            depth: depth.next(),
        }))
    };
    XAttributeValue::Generated {
        template: new_template.clone(),
        consumers: new_attribute_value(new_template),
    }
}
