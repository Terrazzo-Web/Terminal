use quote::quote;

use super::event::process_event;
use super::html_element_visitor::HtmlElementVisitor;

pub struct XElement {
    pub tag_name: Option<proc_macro2::TokenStream>,
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
        let name = name.replace("_", "-");
        match name.as_str() {
            "key" => self.key = quote! { XKey::Named(#value.into()) },
            "before-render" => self.before_render = Some(quote!(#value)),
            "after-render" => self.after_render = Some(quote!(#value)),
            _ => self.attributes.push(quote! {
                gen_attributes.push(XAttribute {
                    name: #name.into(),
                    value: #value.into(),
                });
            }),
        }
    }

    pub fn process_optional_attribute(&mut self, name: &syn::Ident, value: &syn::Expr) {
        if process_event(name, value).is_some() {
            self.events.push(quote! { compile_error!() });
            return;
        };
        let name = name.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        let name = name.strip_suffix("_").unwrap_or(name);
        let name = name.replace("_", "-");
        match name.as_str() {
            "key" => self.key = quote! { compile_error!() },
            "before-render" => self.before_render = Some(quote! { compile_error!() }),
            "after-render" => self.after_render = Some(quote! { compile_error!() }),
            _ => {
                self.attributes.push(quote! {
                    if let Some(value) = #value {
                        gen_attributes.push(XAttribute {
                            name: #name.into(),
                            value: value.into(),
                        });
                    }
                });
            }
        }
    }

    pub fn process_dynamic_attribute(&mut self, name: &syn::Ident, value: &syn::Expr) {
        if process_event(name, value).is_some() {
            self.events.push(quote! { compile_error!() });
            return;
        };
        let name = name.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        let name = name.strip_suffix("_").unwrap_or(name);
        let name = name.replace("_", "-");
        match name.as_str() {
            "key" => self.key = quote! { compile_error!() },
            "before-render" => self.before_render = Some(quote! { compile_error!() }),
            "after-render" => self.after_render = Some(quote! { compile_error!() }),
            _ => {
                self.attributes.push(quote! {
                    gen_attributes.push(XAttribute {
                        name: #name.into(),
                        value: XAttributeValue::Dynamic(
                            (#value).into(),
                        ),
                    });
                });
            }
        }
    }

    pub fn process_dynamic(&mut self, dynamic: &syn::Expr) {
        if self.dynamic.is_some() {
            self.dynamic = Some(quote! {
                compile_error!("Dynamic nodes have a single callback")
            });
            return;
        }
        self.dynamic = Some(quote! {
            XElementValue::Dynamic((#dynamic).into())
        });
    }

    pub fn process_child(
        &mut self,
        html_element_visitor: &mut HtmlElementVisitor,
        child: &syn::Expr,
    ) {
        let child = match child {
            syn::Expr::Call(expr_call)
                if html_element_visitor.get_tag_name(&expr_call.func).is_some() =>
            {
                let child_tag_name = html_element_visitor.get_tag_name(&expr_call.func).unwrap();
                let child = html_element_visitor.process_html_tag(child_tag_name, expr_call);
                quote! { XNode::from(#child) }
            }
            syn::Expr::Macro(expr_macro)
                if html_element_visitor
                    .get_tag_name_from_path(&expr_macro.mac.path)
                    .is_some() =>
            {
                let child_tag_name = html_element_visitor
                    .get_tag_name_from_path(&expr_macro.mac.path)
                    .unwrap();
                let syn::Macro { path, tokens, .. } = &expr_macro.mac;
                let expr_call: syn::ExprCall = syn::parse2(quote! { #path(#tokens) }).unwrap();
                let child = html_element_visitor.process_html_tag(child_tag_name, &expr_call);
                quote! { XNode::from(#child) }
            }
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(string),
                ..
            }) => quote! { XNode::from(XText(format!(#string).into())) },
            child => quote! { XNode::from(#child) },
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
