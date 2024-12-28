use quote::ToTokens as _;
use syn::visit_mut::VisitMut as _;

use self::html_element_visitor::HtmlElementVisitor;
use crate::arguments::MacroArgs;

mod element;
mod event;
mod html_element_visitor;
mod tests;

pub fn html(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let args = MacroArgs::parse2(attr)?;
    let mut item: syn::Item = syn::parse2(item)?;
    let mut html_element_visitor = HtmlElementVisitor {
        args,
        success: Ok(()),
    };
    html_element_visitor.visit_item_mut(&mut item);
    let () = html_element_visitor.success?;
    return Ok(item.into_token_stream());
}
