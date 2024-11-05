#![cfg(test)]

use quote::quote;

use super::template;
use crate::item_to_string;

#[test]
fn simple_node() -> syn::Result<()> {
    let sample = quote! {
        fn sample() {
            div(key = "root", "Root text")
        }
    };
    let expected = r#"
fn sample(generated_template: XTemplate) -> Consumers {
    let generated_template1 = generated_template.clone();
    make_reactive_closure()
        .named("sample")
        .closure(move || {
            let generated_template = generated_template.clone();
            let generated_body = move || { div(key = "root", "Root text") };
            generated_template.apply(generated_body)
        })
        .register(generated_template1)
}"#;
    let actual = template(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn with_signal() -> syn::Result<()> {
    let sample = quote! {
        fn sample(#[signal] signal: String) {
            div(key = "root", "Root text")
        }
    };
    let expected = r#"
fn sample(generated_template: XTemplate, signal: XSignal<String>) -> Consumers {
    let generated_template1 = generated_template.clone();
    make_reactive_closure()
        .named("sample")
        .closure(move || {
            let generated_template = generated_template.clone();
            move |signal: String| {
                let generated_body = move || { div(key = "root", "Root text") };
                generated_template.apply(generated_body)
            }
        })
        .bind(signal)
        .register(generated_template1)
}"#;
    let actual = template(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn with_2_signals() -> syn::Result<()> {
    let sample = quote! {
        fn sample(#[signal] signal1: String, #[signal] signal2: &'static str) {
            div(key = "root", "{signal1}", "{signal2}")
        }
    };
    let expected = r#"
fn sample(
    generated_template: XTemplate,
    signal1: XSignal<String>,
    signal2: XSignal<&'static str>,
) -> Consumers {
    let generated_template1 = generated_template.clone();
    make_reactive_closure()
        .named("sample")
        .closure(move || {
            let generated_template = generated_template.clone();
            move |signal1: String| {
                move |signal2: &'static str| {
                    let generated_body = move || {
                        div(key = "root", "{signal1}", "{signal2}")
                    };
                    generated_template.apply(generated_body)
                }
            }
        })
        .bind(signal1)
        .bind(signal2)
        .register(generated_template1)
}"#;
    let actual = template(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn with_2_mutable_signals() -> syn::Result<()> {
    let sample = quote! {
        fn sample(
            #[signal] mut signal1: String,
            #[signal] mut signal2: &'static str,
            constant: i32,
        ) {
            div(key = "root", "{signal1}", "{signal2}")
        }
    };
    let expected = r#"
fn sample(
    generated_template: XTemplate,
    signal1: XSignal<String>,
    signal2: XSignal<&'static str>,
    constant: i32,
) -> Consumers {
    let generated_template1 = generated_template.clone();
    let signal1_mut = signal1.clone();
    let signal2_mut = signal2.clone();
    make_reactive_closure()
        .named("sample")
        .closure(move || {
            let generated_template = generated_template.clone();
            let signal1_mut = MutableSignal::from(&signal1_mut);
            let signal2_mut = MutableSignal::from(&signal2_mut);
            let constant = constant.clone();
            move |signal1: String| {
                move |signal2: &'static str| {
                    let generated_body = move || {
                        div(key = "root", "{signal1}", "{signal2}")
                    };
                    generated_template.apply(generated_body)
                }
            }
        })
        .bind(signal1)
        .bind(signal2)
        .register(generated_template1)
}"#;
    let actual = template(quote! {}, sample)?;
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}
