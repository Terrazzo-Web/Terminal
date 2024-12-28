use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::Ident;

#[proc_macro_attribute]
pub fn named(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr.into()) {
        Ok(args) => args,
        Err(error) => {
            return TokenStream::from(darling::Error::from(error).write_errors());
        }
    };
    let attr_args = match NamedMacroArgs::from_list(&attr_args) {
        Ok(args) => args,
        Err(error) => {
            return TokenStream::from(error.write_errors());
        }
    };

    let item: syn::Item = match syn::parse(tokens.clone()) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error().into(),
    };

    let crate_name = attr_args.crate_override.as_deref().unwrap_or("named");
    let crate_name = format_ident!("{}", crate_name);

    let name = match item {
        syn::Item::Struct(item_struct) => process_struct(&crate_name, item_struct),
        syn::Item::Enum(item_enum) => process_enum(&crate_name, item_enum),
        syn::Item::Fn(item_fn) => process_fn(item_fn),
        _ => return quote! { compile_error!("Unexpected item kind"); }.into(),
    };

    if attr_args.debug {
        println!("\nGenerated:\n{name}\n");
    }

    let mut tokens = tokens;
    tokens.extend(name);
    return tokens;
}

fn process_struct(crate_name: &Ident, item_struct: syn::ItemStruct) -> TokenStream {
    let syn::ItemStruct {
        ident, generics, ..
    } = item_struct;
    let name = ident.to_string();
    let param_names_only = param_names_only(&generics);
    quote! {
        impl #generics #crate_name::NamedType for #ident #param_names_only {
            fn type_name() -> &'static str {
                return #name;
            }
        }
    }
    .into()
}

fn process_enum(crate_name: &Ident, item_enum: syn::ItemEnum) -> TokenStream {
    let syn::ItemEnum {
        ident,
        generics,
        variants,
        ..
    } = item_enum;
    let cases: Vec<_> = variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            let name = ident.to_string();
            quote! { Self::#ident { .. } => #name, }
        })
        .collect();

    let param_names_only = param_names_only(&generics);
    let name = ident.to_string();
    quote! {
        impl #generics #crate_name::NamedType for #ident #param_names_only {
            fn type_name() -> &'static str {
                return #name;
            }
        }
        impl #generics #crate_name::NamedEnumValues for #ident #param_names_only {
            fn name(&self) -> &'static str {
                match self {
                    #(#cases)*
                }
            }
        }
    }
    .into()
}

fn process_fn(item_fn: syn::ItemFn) -> TokenStream {
    let name = item_fn.sig.ident.to_string();
    let vis = item_fn.vis;
    let ident = format_ident!("{}", name.to_uppercase());
    quote! { #vis const #ident : &'static str = #name; }.into()
}

#[derive(Debug, FromMeta)]
struct NamedMacroArgs {
    #[darling(default)]
    debug: bool,

    #[darling(default)]
    crate_override: Option<String>,
}

fn param_names_only(generics: &syn::Generics) -> syn::Generics {
    let mut generics = generics.clone();
    for param in &mut generics.params {
        match param {
            syn::GenericParam::Lifetime(syn::LifetimeParam {
                attrs,
                colon_token,
                bounds,
                ..
            }) => {
                *attrs = vec![];
                *colon_token = None;
                *bounds = syn::punctuated::Punctuated::default()
            }
            syn::GenericParam::Type(syn::TypeParam {
                attrs,
                colon_token,
                bounds,
                eq_token,
                default,
                ..
            }) => {
                *attrs = vec![];
                *colon_token = None;
                *bounds = syn::punctuated::Punctuated::default();
                *eq_token = None;
                *default = None;
            }
            syn::GenericParam::Const(syn::ConstParam {
                attrs,
                eq_token,
                default,
                ..
            }) => {
                *attrs = vec![];
                *eq_token = None;
                *default = None;
            }
        }
    }
    generics
}
