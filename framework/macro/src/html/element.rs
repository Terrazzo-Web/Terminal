use quote::quote;
use syn::spanned::Spanned;

use super::event::process_event;
use super::html_element_visitor::HtmlElementVisitor;

pub struct XElement {
    pub tag_name: proc_macro2::TokenStream,
    pub key: proc_macro2::TokenStream,
    pub attributes: Vec<proc_macro2::TokenStream>,
    pub events: Vec<proc_macro2::TokenStream>,
    pub children: Vec<proc_macro2::TokenStream>,
    pub dynamic: Option<proc_macro2::TokenStream>,
    pub before_render: Option<proc_macro2::TokenStream>,
    pub after_render: Option<proc_macro2::TokenStream>,
}

impl XElement {
    pub fn process_attribute(&mut self, name: &syn::Ident, value: &syn::Expr) {
        if let Some(event) = process_event(name, value) {
            self.events.push(event);
            return;
        };
        let name = name.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        let name = name.strip_suffix("_").unwrap_or(name);
        match name {
            "tag_name" => self.tag_name = quote! { #value.into() },
            "key" => self.key = quote! { XKey::Named(#value.into()) },
            "before_render" => self.before_render = Some(quote!(#value)),
            "after_render" => self.after_render = Some(quote!(#value)),
            _ => self.attributes.push(quote! {
                XAttribute {
                    name: #name.into(),
                    value: #value.into(),
                }
            }),
        }
    }

    pub fn process_dynamic(&mut self, dynamic: &syn::Expr) {
        self.dynamic = Some(quote! {
            value: XElementValue::Dynamic((#dynamic).into())
        });
    }

    pub fn process_child(
        &mut self,
        html_element_visitor: &mut HtmlElementVisitor,
        child: &syn::Expr,
    ) {
        let child = match child {
            syn::Expr::Call(expr_call) => {
                let Some(child_tag_name) = html_element_visitor.get_tag_name(&expr_call.func)
                else {
                    html_element_visitor.success =
                        Err(syn::Error::new(expr_call.span(), "Not a known HTML tag"));
                    return;
                };
                let child = html_element_visitor.process_html_tag(child_tag_name, expr_call);
                quote! { XNode::from(#child) }
            }
            syn::Expr::Macro(expr_macro) => {
                let Some(child_tag_name) =
                    html_element_visitor.get_tag_name_from_path(&expr_macro.mac.path)
                else {
                    html_element_visitor.success =
                        Err(syn::Error::new(expr_macro.span(), "Not a known HTML tag"));
                    return;
                };
                let syn::Macro { path, tokens, .. } = &expr_macro.mac;
                let expr_call: syn::ExprCall = syn::parse2(quote! { #path(#tokens) }).unwrap();
                let child = html_element_visitor.process_html_tag(child_tag_name, &expr_call);
                quote! { XNode::from(#child) }
            }
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(string),
                ..
            }) => quote! { XNode::from(XText(format!(#string).into())) },
            _ => {
                html_element_visitor.success =
                    Err(syn::Error::new(child.span(), "Invalid child node"));
                return;
            }
        };
        self.children.push(quote! {
            gen_children.push(#child);
        });
    }

    pub fn process_children(&mut self, children: &syn::Expr) {
        self.children.push(quote! {
            gen_children.extend(#children.into_iter().map(XNode::from));
        });
    }
}
