use std::collections::HashMap;

use tracing::trace;
use tracing::warn;
use web_sys::Element;

use super::XAttribute;
use crate::attribute::XAttributeValue;
use crate::signal::depth::Depth;

pub fn merge(
    depth: Depth,
    new_attributes: &mut [XAttribute],
    old_attributes: &mut [XAttribute],
    element: &Element,
) {
    trace!(
        new_count = new_attributes.len(),
        old_count = old_attributes.len(),
        "Attributes"
    );

    let mut old_attributes_map = HashMap::new();
    for old_attribute in old_attributes {
        old_attributes_map.insert(
            std::mem::take(&mut old_attribute.name),
            std::mem::replace(&mut old_attribute.value, XAttributeValue::Static("".into())),
        );
    }

    for new_attribute in new_attributes {
        let attribute_name = &new_attribute.name;
        let old_attribute_value = old_attributes_map.remove(attribute_name);
        new_attribute.merge(depth, old_attribute_value, element);
    }

    for removed_old_attribute_name in old_attributes_map.keys() {
        match element.remove_attribute(removed_old_attribute_name.as_str()) {
            Ok(()) => {
                trace! { "Removed attribute {}", removed_old_attribute_name };
            }
            Err(error) => {
                warn! { "Removed attribute {} failed: {error:?}", removed_old_attribute_name };
            }
        }
    }
}
