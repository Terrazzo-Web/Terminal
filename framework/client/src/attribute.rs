use std::rc::Rc;

use named::named;
use named::NamedType as _;
use tracing::trace;
use tracing::warn;
use wasm_bindgen::JsCast as _;
use web_sys::Element;
use web_sys::HtmlElement;

use self::inner::AttributeTemplateInner;
use crate::debug_correlation_id::DebugCorrelationId;
use crate::prelude::OrElseLog;
use crate::signal::depth::Depth;
use crate::signal::reactive_closure::reactive_closure_builder::Consumers;
use crate::string::XString;
use crate::template::IsTemplate;
use crate::template::IsTemplated;

#[named]
pub struct XAttribute {
    pub name: XAttributeName,
    pub value: XAttributeValue,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum XAttributeName {
    Attribute(XString),
    Style(XString),
}

pub enum XAttributeValue {
    Null,
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

    use super::XAttributeName;
    use super::XAttributeTemplate;
    use crate::debug_correlation_id::DebugCorrelationId;
    use crate::signal::depth::Depth;

    pub struct AttributeTemplateInner {
        pub element: Element,
        pub attribute_name: XAttributeName,
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

    fn apply<R: Into<Self::Value>>(self, new: impl FnOnce() -> R) {
        let mut new = XAttribute {
            name: self.attribute_name.clone(),
            value: new().into(),
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

impl<T> From<Option<T>> for XAttributeValue
where
    XString: From<T>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Static(value.into()),
            None => Self::Null,
        }
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
            XAttributeValue::Null => {
                merge_static_atttribute(element, attribute_name, None, old_attribute_value);
            }
            XAttributeValue::Static(new_attribute_value) => {
                merge_static_atttribute(
                    element,
                    attribute_name,
                    Some(new_attribute_value),
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
    attribute_name: &XAttributeName,
    new_value: Option<&XString>,
    old_value: Option<XAttributeValue>,
) {
    if let Some((XAttributeValue::Static(old_attribute_value), new_value)) =
        old_value.as_ref().zip(new_value)
    {
        if new_value == old_attribute_value {
            trace!("Attribute '{attribute_name}' is still '{new_value}'");
            return;
        }
    }
    drop(old_value);
    let Some(new_value) = new_value else {
        match attribute_name {
            XAttributeName::Attribute(name) => match element.remove_attribute(name.as_str()) {
                Ok(()) => trace!("Removed attribute {name}"),
                Err(error) => warn!("Removed attribute {name} failed: {error:?}"),
            },
            XAttributeName::Style(name) => {
                let html_element: &HtmlElement = element.dyn_ref().or_throw("HtmlElement");
                let style = html_element.style();
                match style.remove_property(name) {
                    Ok(value) => trace!("Removed style {name}: {value}"),
                    Err(error) => warn!("Removed style {name} failed: {error:?}"),
                }
            }
        }
        return;
    };
    match attribute_name {
        XAttributeName::Attribute(name) => match element.set_attribute(name, new_value) {
            Ok(()) => trace!("Set attribute '{name}' to '{new_value}'"),
            Err(error) => warn!("Set attribute '{name}' to '{new_value}' failed: {error:?}"),
        },
        XAttributeName::Style(name) => {
            let html_element: &HtmlElement = element.dyn_ref().or_throw("HtmlElement");
            let style = html_element.style();
            match style.set_property(name, new_value) {
                Ok(()) => trace!("Set style {name}: {new_value}"),
                Err(error) => warn!("Set style {name}: {new_value} failed: {error:?}"),
            }
        }
    }
}

fn merge_dynamic_atttribute(
    depth: Depth,
    element: &Element,
    attribute_name: &XAttributeName,
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

impl<T> From<T> for XAttributeName
where
    T: Into<XString>,
{
    fn from(value: T) -> Self {
        Self::Attribute(value.into())
    }
}

impl std::fmt::Display for XAttributeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attribute(name) => std::fmt::Display::fmt(name, f),
            Self::Style(name) => write!(f, "style::{}", name),
        }
    }
}
