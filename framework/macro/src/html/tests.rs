#![cfg(test)]

use quote::quote;

use super::html;
use crate::item_to_string;

#[test]
fn simple_node() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(key = "root", "Root text")
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn child() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            return html::div(
                key = "root",
                "Root text",
                span(key = "inner", "Paragraph 1", "Paragraph 2"),
            );
        }
    };
    let expected = r#"
fn sample() -> XElement {
    return {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        gen_children
            .push(
                XNode::from({
                    let mut gen_attributes = vec![];
                    let mut gen_children = vec![];
                    gen_children.push(XNode::from(XText(format!("Paragraph 1").into())));
                    gen_children.push(XNode::from(XText(format!("Paragraph 2").into())));
                    XElement {
                        tag_name: Some("span".into()),
                        key: XKey::Named("inner".into()),
                        value: XElementValue::Static {
                            attributes: gen_attributes,
                            children: gen_children,
                            events: vec![],
                        },
                        before_render: None,
                        after_render: None,
                    }
                }),
            );
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn children() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            let child1 = span("Child1");
            let child2 = span("Child2");
            let children = [child1, child2];
            return html::div(
                key = "root",
                "Root text",
                children..,
            );
        }
    };
    let expected = r#"
fn sample() -> XElement {
    let child1 = {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Child1").into())));
        XElement {
            tag_name: Some("span".into()),
            key: XKey::default(),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
    let child2 = {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Child2").into())));
        XElement {
            tag_name: Some("span".into()),
            key: XKey::default(),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
    let children = [child1, child2];
    return {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        gen_children.extend(children.into_iter().map(XNode::from));
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn invalid_child() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            let someNode = someNode()..;
            div(key = "root", "Text", someNode)
        }
    };
    let expected = r#"
fn sample() -> XElement {
    let someNode = someNode()..;
    {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Text").into())));
        gen_children.push(XNode::from(someNode));
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn attribute() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(
                key = "root",
                "Root text",
                class = "base",
                style = format!("width: {}%", 100),
            )
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        let mut gen_attributes = vec![];
        gen_attributes
            .push(XAttribute {
                name: "class".into(),
                value: "base".into(),
            });
        gen_attributes
            .push(XAttribute {
                name: "style".into(),
                value: format!("width: {}%", 100).into(),
            });
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn optional_attribute() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(
                key = "root",
                "Root text",
                class = "base",
                style |= Some(format!("width: {}%", 100)),
                data_custom |= Some("custom attribute"),
                data_custom |= if true { Some("y") } else { None },
            )
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        let mut gen_attributes = vec![];
        gen_attributes
            .push(XAttribute {
                name: "class".into(),
                value: "base".into(),
            });
        if let Some(value) = Some(format!("width: {}%", 100)) {
            gen_attributes
                .push(XAttribute {
                    name: "style".into(),
                    value: value.into(),
                });
        }
        if let Some(value) = Some("custom attribute") {
            gen_attributes
                .push(XAttribute {
                    name: "data-custom".into(),
                    value: value.into(),
                });
        }
        if let Some(value) = if true { Some("y") } else { None } {
            gen_attributes
                .push(XAttribute {
                    name: "data-custom".into(),
                    value: value.into(),
                });
        }
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn index_keys() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            return html::div(span("Paragraph 1"), span("Paragraph 2"));
        }
    };
    let expected = r#"
fn sample() -> XElement {
    return {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children
            .push(
                XNode::from({
                    let mut gen_attributes = vec![];
                    let mut gen_children = vec![];
                    gen_children.push(XNode::from(XText(format!("Paragraph 1").into())));
                    XElement {
                        tag_name: Some("span".into()),
                        key: XKey::default(),
                        value: XElementValue::Static {
                            attributes: gen_attributes,
                            children: gen_children,
                            events: vec![],
                        },
                        before_render: None,
                        after_render: None,
                    }
                }),
            );
        gen_children
            .push(
                XNode::from({
                    let mut gen_attributes = vec![];
                    let mut gen_children = vec![];
                    gen_children.push(XNode::from(XText(format!("Paragraph 2").into())));
                    XElement {
                        tag_name: Some("span".into()),
                        key: XKey::default(),
                        value: XElementValue::Static {
                            attributes: gen_attributes,
                            children: gen_children,
                            events: vec![],
                        },
                        before_render: None,
                        after_render: None,
                    }
                }),
            );
        XElement {
            tag_name: Some("div".into()),
            key: XKey::default(),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn dynamic() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(|element| do_template(element))
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        XElement {
            tag_name: Some("div".into()),
            key: XKey::default(),
            value: XElementValue::Dynamic((|element| do_template(element)).into()),
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn dynamic_duplicate_callback() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(
                |element| do_template(element),
                |element| do_template(element),
            )
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        XElement {
            tag_name: Some("div".into()),
            key: XKey::default(),
            value: compile_error!("Dynamic nodes have a single callback"),
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn dynamic_invalid() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div(class = "stylish", |element| do_template(element))
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        compile_error!("Properties of dynamic nodes cannot be defined at the call site");
        XElement {
            tag_name: Some("div".into()),
            key: XKey::default(),
            value: XElementValue::Dynamic((|element| do_template(element)).into()),
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn child_macros() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            return div! {
                key = "root",
                "Root text",
                span! {
                    key = "inner",
                    "Paragraph 1",
                    "Paragraph 2",
                },
            };
        }
    };
    let expected = r#"
fn sample() -> XElement {
    return {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        gen_children
            .push(
                XNode::from({
                    let mut gen_attributes = vec![];
                    let mut gen_children = vec![];
                    gen_children.push(XNode::from(XText(format!("Paragraph 1").into())));
                    gen_children.push(XNode::from(XText(format!("Paragraph 2").into())));
                    XElement {
                        tag_name: Some("span".into()),
                        key: XKey::Named("inner".into()),
                        value: XElementValue::Static {
                            attributes: gen_attributes,
                            children: gen_children,
                            events: vec![],
                        },
                        before_render: None,
                        after_render: None,
                    }
                }),
            );
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    };
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn child_macros2() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            div! {
                key = "root",
                "Root text",
                span! {
                    key = "inner",
                    "Paragraph 1",
                    "Paragraph 2",
                },
            }
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        gen_children
            .push(
                XNode::from({
                    let mut gen_attributes = vec![];
                    let mut gen_children = vec![];
                    gen_children.push(XNode::from(XText(format!("Paragraph 1").into())));
                    gen_children.push(XNode::from(XText(format!("Paragraph 2").into())));
                    XElement {
                        tag_name: Some("span".into()),
                        key: XKey::Named("inner".into()),
                        value: XElementValue::Static {
                            attributes: gen_attributes,
                            children: gen_children,
                            events: vec![],
                        },
                        before_render: None,
                        after_render: None,
                    }
                }),
            );
        XElement {
            tag_name: Some("div".into()),
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn tag() -> syn::Result<()> {
    let sample = quote! {
        fn sample() -> XElement {
            tag(key = "root", "Root text")
        }
    };
    let expected = r#"
fn sample() -> XElement {
    {
        let mut gen_attributes = vec![];
        let mut gen_children = vec![];
        gen_children.push(XNode::from(XText(format!("Root text").into())));
        XElement {
            tag_name: None,
            key: XKey::Named("root".into()),
            value: XElementValue::Static {
                attributes: gen_attributes,
                children: gen_children,
                events: vec![],
            },
            before_render: None,
            after_render: None,
        }
    }
}"#;
    let actual = html(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}
