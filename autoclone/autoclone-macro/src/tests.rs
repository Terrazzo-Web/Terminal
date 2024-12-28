#![cfg(test)]

use quote::quote;

use crate::item_to_string;

#[test]
fn autoclone() -> syn::Result<()> {
    let sample = quote! {
        fn sample() {
            let shared = Arc::new("something".to_string());
            let f = move || {
                autoclone!(shared);
                println!("{shared}");
            };
            f();
        }
    };
    let expected = r#"
fn sample() {
    let shared = Arc::new("something".to_string());
    let f = {
        let shared = shared.to_owned();
        move || {
            println!("{shared}");
        }
    };
    f();
}"#;
    let actual = super::autoclone2(quote! {}, sample);
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn multiple() -> syn::Result<()> {
    let sample = quote! {
        fn sample() {
            let shared1 = Arc::new("something1".to_string());
            let shared2 = Arc::new("something2".to_string());
            let f = move || {
                autoclone!(shared1);
                autoclone!(shared2);
                println!("{shared1} {shared2}");
            };
            f();
        }
    };
    let expected = r#"
fn sample() {
    let shared1 = Arc::new("something1".to_string());
    let shared2 = Arc::new("something2".to_string());
    let f = {
        let shared1 = shared1.to_owned();
        let shared2 = shared2.to_owned();
        move || {
            println!("{shared1} {shared2}");
        }
    };
    f();
}"#;
    let actual = super::autoclone2(quote! {}, sample);
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn multiple_inline() -> syn::Result<()> {
    let sample = quote! {
        fn sample() {
            let shared1 = Arc::new("something1".to_string());
            let shared2 = Arc::new("something2".to_string());
            let f = move || {
                autoclone!(shared1, shared2);
                println!("{shared1} {shared2}");
            };
            f();
        }
    };
    let expected = r#"
fn sample() {
    let shared1 = Arc::new("something1".to_string());
    let shared2 = Arc::new("something2".to_string());
    let f = {
        let shared1 = shared1.to_owned();
        let shared2 = shared2.to_owned();
        move || {
            println!("{shared1} {shared2}");
        }
    };
    f();
}"#;
    let actual = super::autoclone2(quote! {}, sample);
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn inner() -> syn::Result<()> {
    let sample = quote! {
        fn outer() {
            let shared1 = Arc::new("something1".to_string());
            let outer_f = move || {
                autoclone!(shared1);
                let shared2 = Arc::new("something2".to_string());
                let inner_f = move || {
                    autoclone!(shared2);
                    println!("{shared1} {shared2}");
                };
                inner_f();
            };
            outer_f();
        }
    };
    let expected = r#"
fn outer() {
    let shared1 = Arc::new("something1".to_string());
    let outer_f = {
        let shared1 = shared1.to_owned();
        move || {
            let shared2 = Arc::new("something2".to_string());
            let inner_f = {
                let shared2 = shared2.to_owned();
                move || {
                    println!("{shared1} {shared2}");
                }
            };
            inner_f();
        }
    };
    outer_f();
}"#;
    let actual = super::autoclone2(quote! {}, sample);
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}

#[test]
fn unused() {
    let sample = quote! {
        fn sample() {}
    };
    let actual = super::autoclone2(quote! {}, sample);
    assert_eq!(
        actual.to_string(),
        quote! {compile_error!("autoclone is not used")}.to_string()
    );
}

#[test]
fn with_async() -> syn::Result<()> {
    let sample = quote! {
        fn outer() {
            let shared1 = Arc::new("something1".to_string());
            let outer_f = async {
                autoclone!(shared1);
                let shared2 = Arc::new("something2".to_string());
                let inner_f = async {
                    autoclone!(shared2);
                    println!("{shared1} {shared2}");
                };
                inner_f();
            };
            outer_f();
        }
    };
    let expected = r#"
fn outer() {
    let shared1 = Arc::new("something1".to_string());
    let outer_f = {
        let shared1 = shared1.to_owned();
        async {
            let shared2 = Arc::new("something2".to_string());
            let inner_f = {
                let shared2 = shared2.to_owned();
                async {
                    println!("{shared1} {shared2}");
                }
            };
            inner_f();
        }
    };
    outer_f();
}"#;
    let actual = super::autoclone2(quote! {}, sample);
    let actual = item_to_string(&syn::parse2(actual)?);
    if expected.trim() != actual.trim() {
        println!("{}", actual);
        assert!(false);
    }
    Ok(())
}
