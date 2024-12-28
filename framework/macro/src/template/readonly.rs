use quote::quote;

pub fn process_readonly_input(
    copies: &mut Vec<proc_macro2::TokenStream>,
    input: &mut syn::PatType,
) {
    let pat = &input.pat;
    copies.push(quote! { let #pat = #pat.clone(); });
}
