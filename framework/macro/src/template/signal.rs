use quote::format_ident;
use quote::quote;

use crate::is_path;

pub fn process_signal_input(
    prelude: &mut Vec<proc_macro2::TokenStream>,
    copies: &mut Vec<proc_macro2::TokenStream>,
    input: &mut syn::PatType,
) -> bool {
    if !has_signal_attr(input) {
        return false;
    }
    if let syn::Pat::Ident(ident) = &mut *input.pat {
        if ident.mutability.take().is_some() {
            let ident = &ident.ident;
            let alias = format_ident!("{ident}_mut");
            prelude.push(quote! { let #alias = #ident.clone(); });
            copies.push(quote! { let #alias = MutableSignal::from(&#alias); });
        }
    }
    return true;
}

fn has_signal_attr(input: &mut syn::PatType) -> bool {
    for i in 0..input.attrs.len() {
        let attribute = &input.attrs[i];
        match &attribute.meta {
            syn::Meta::Path(path) if is_path(path, "signal") => (),
            _ => continue,
        };
        input.attrs.remove(i);
        return true;
    }
    return false;
}
