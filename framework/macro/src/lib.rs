use quote::format_ident;

mod arguments;
mod html;
mod template;

#[proc_macro_attribute]
pub fn html(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    self::html::html(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn template(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    self::template::template(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn item_to_string(item: &syn::Item) -> String {
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![item.clone()],
    })
}

fn is_path(path: &syn::Path, expected: &'static str) -> bool {
    let syn::Path {
        leading_colon: None,
        segments,
    } = path
    else {
        return false;
    };
    let Some(segment) = segments.last() else {
        return false;
    };
    if segments.len() != 1 {
        return false;
    }
    let syn::PathArguments::None = segment.arguments else {
        return false;
    };
    return segment.ident == format_ident!("{}", expected);
}
