use quote::quote;
use quote::quote_spanned;
use quote::ToTokens as _;
use readonly::process_readonly_input;
use syn::spanned::Spanned;

use self::signal::process_signal_input;
use crate::arguments::MacroArgs;

mod readonly;
mod signal;
mod tests;

pub fn template(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let args = MacroArgs::parse2(attr)?;
    let mut item_fn: syn::ItemFn = syn::parse2(item)?;

    let name = item_fn.sig.ident.to_string();
    let mut prelude = vec![];
    let mut copies = vec![];
    let mut body = item_fn.block.to_token_stream();
    let mut bind_signals = vec![];

    body = quote! {
        let generated_body = move || #body;
        generated_template.apply(generated_body)
    };
    let mut inputs: Vec<_> = item_fn.sig.inputs.iter_mut().collect();
    inputs.reverse();
    for input in inputs {
        let syn::FnArg::Typed(input) = input else {
            continue;
        };
        let is_signal = process_signal_input(&mut prelude, &mut copies, input);
        if is_signal {
            body = quote! { move | #input | {
                #body
            }};
            {
                let pat = &input.pat;
                bind_signals.push(quote! { .bind(#pat) });
            }
            {
                let ty = &mut input.ty;
                *ty = Box::new(syn::parse2(quote! { XSignal<#ty> })?);
            }
        } else {
            process_readonly_input(&mut copies, input);
        }
    }

    prelude.reverse();
    copies.reverse();
    bind_signals.reverse();

    let element: syn::FnArg = {
        let span = item_fn.sig.ident.span();
        syn::parse2(quote_spanned! {span=> generated_template: XTemplate})?
    };
    item_fn.sig.inputs.insert(0, element);

    let block = quote! {
        {
            let generated_template1 = generated_template.clone();
            #(#prelude)*
            make_reactive_closure()
                .named(#name)
                .closure(move || {
                    let generated_template = generated_template.clone();
                    #(#copies)*
                    #body
                })
                #(#bind_signals)*
                .register(generated_template1)
        }
    };
    item_fn.block = Box::new(syn::parse2(block)?);
    item_fn.sig.output = syn::ReturnType::Type(
        syn::Token![->](item_fn.sig.output.span()),
        Box::new(syn::parse2(quote! { Consumers }).unwrap()),
    );

    if args.debug {
        println!("{}\n", item_to_string(&item_fn));
    }
    Ok(item_fn.to_token_stream())
}

fn item_to_string(item: &syn::ItemFn) -> String {
    crate::item_to_string(&syn::Item::Fn(item.clone()))
}
