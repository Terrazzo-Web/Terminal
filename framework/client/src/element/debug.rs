#[cfg(not(feature = "concise_traces"))]
mod verbose {
    use std::ops::DerefMut;

    use crate::attribute::XAttributeValue;
    use crate::element::template::XTemplate;
    use crate::element::XElement;
    use crate::element::XElementValue;
    use crate::node::XNode;
    use crate::template::IsTemplate as _;

    impl std::fmt::Debug for XElement {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let mut b = DebugStringBuilder::default();
            self.print_debug(&mut b);
            write!(f, "{}", b.buffer)
        }
    }

    #[derive(Default)]
    struct DebugStringBuilder {
        padding: String,
        buffer: String,
    }

    impl DebugStringBuilder {
        fn writeln(&mut self, s: impl std::fmt::Display) -> &mut Self {
            self.buffer += &format!("{}{s}\n", self.padding);
            self
        }

        fn indent(&mut self, count: usize) -> impl DerefMut<Target = &mut DebugStringBuilder> {
            self.padding += &String::from_iter(std::iter::repeat(' ').take(count));
            scopeguard::guard(self, move |b| {
                b.padding.drain((b.padding.len() - count)..);
            })
        }
    }

    trait PrintDebug {
        fn print_debug(&self, b: &mut DebugStringBuilder);
    }

    impl PrintDebug for XElement {
        fn print_debug(&self, b: &mut DebugStringBuilder) {
            let mut rest = String::new();
            match &self.value {
                XElementValue::Static {
                    attributes, events, ..
                } => {
                    for attribute in attributes {
                        match &attribute.value {
                            XAttributeValue::Static(value) => {
                                rest += &format!(" {}={:?}", attribute.name, value)
                            }
                            XAttributeValue::Dynamic { .. } => {
                                rest += &format!(" {}=<Dynamic>", attribute.name)
                            }
                            XAttributeValue::Generated { .. } => {
                                rest += &format!(" {}=<Generated>", attribute.name)
                            }
                        }
                    }
                    for event in events {
                        rest += &format!(" on:{}=[callback]", event.event_type);
                    }
                }
                XElementValue::Dynamic { .. } => (),
                XElementValue::Generated { .. } => (),
            }
            b.writeln(format!(
                "<{tag_name} key={key:?}{rest}>",
                tag_name = self.tag_name,
                key = self.key
            ));
            {
                let mut b = b.indent(4);
                match &self.value {
                    XElementValue::Static { children, .. } => {
                        for child in children {
                            child.print_debug(&mut b);
                        }
                    }
                    XElementValue::Dynamic { .. } => {
                        b.writeln("[dynamic]");
                    }
                    XElementValue::Generated { template, .. } => {
                        b.writeln(format!("<template {}>", template.debug_id()));
                        {
                            let mut b = b.indent(4);
                            template.print_debug(&mut b);
                        }
                        b.writeln(format!("</template {}>", template.debug_id()));
                    }
                }
            }
            b.writeln(format!("</{tag_name}>", tag_name = self.tag_name));
        }
    }

    impl PrintDebug for XNode {
        fn print_debug(&self, b: &mut DebugStringBuilder) {
            match self {
                XNode::Element(xelement) => {
                    xelement.print_debug(b);
                }
                XNode::Text(xtext) => {
                    b.writeln(format!("{:?}", xtext.0.to_string()));
                }
            }
        }
    }

    impl PrintDebug for XTemplate {
        fn print_debug(&self, b: &mut DebugStringBuilder) {
            self.with_old(|element| {
                if let Some(element) = element {
                    element.print_debug(b);
                } else {
                    b.writeln("[empty]");
                }
            });
        }
    }

    #[cfg(test)]
    mod tests {
        use terrazzo_macro::html;
        use terrazzo_macro::template;

        use crate::prelude::*;

        #[test]
        fn element() {
            #[html]
            fn html() -> XElement {
                div(
                    key = "root",
                    class = "root-css-style",
                    "Text",
                    ul(
                        li(key = "1", "First"),
                        li(key = "2", "Second"),
                        li(key = "3", "Third"),
                    ),
                    p(|t| child(t)),
                    data_dyn_attribute %= |t| dyn_attribute(t),
                )
            }

            #[html]
            #[template]
            fn child() -> XElement {
                div(key = "child", class = "child-css-style", span("Child"))
            }

            #[template]
            fn dyn_attribute() -> XAttributeValue {
                "custom-value".into()
            }

            let expected = r#"
<div key='root' class=Str("root-css-style") data-dyn-attribute=<Dynamic>>
    "Text"
    <ul key=#0>
        <li key='1'>
            "First"
        </li>
        <li key='2'>
            "Second"
        </li>
        <li key='3'>
            "Third"
        </li>
    </ul>
    <p key=#0>
        [dynamic]
    </p>
</div>"#;
            let actual = format!("{:?}", html());
            if expected.trim() != actual.trim() {
                println!("{}", actual);
                assert!(false);
            }
        }
    }
}

#[cfg(feature = "concise_traces")]
mod concise {
    use named::NamedType as _;

    use crate::element::XElement;

    impl std::fmt::Debug for XElement {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct(XElement::type_name())
                .field("key", &self.key)
                .field("tag_name", &self.tag_name)
                .finish()
        }
    }
}
