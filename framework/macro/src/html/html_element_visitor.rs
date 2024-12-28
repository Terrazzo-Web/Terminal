use quote::quote;
use syn::visit_mut::VisitMut;

use super::element::XElement;
use crate::arguments::MacroArgs;

pub struct HtmlElementVisitor {
    pub args: MacroArgs,
    pub success: syn::Result<()>,
}

impl VisitMut for HtmlElementVisitor {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Call(expr_call) = expr {
            if let Some(tag_name) = self.get_tag_name(&expr_call.func) {
                *expr = self.process_html_tag(tag_name, expr_call);
            }
        }
        if let syn::Expr::Macro(expr_macro) = expr {
            if let Some(tag_name) = self.get_tag_name_from_path(&expr_macro.mac.path) {
                let syn::Macro { path, tokens, .. } = &expr_macro.mac;
                let expr_call: syn::ExprCall = syn::parse2(quote! { #path(#tokens) }).unwrap();
                *expr = self.process_html_tag(tag_name, &expr_call);
            }
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_stmt_mut(&mut self, stmt: &mut syn::Stmt) {
        if let syn::Stmt::Macro(stmt_macro) = stmt {
            if let Some(tag_name) = self.get_tag_name_from_path(&stmt_macro.mac.path) {
                let syn::Macro { path, tokens, .. } = &stmt_macro.mac;
                let expr_call: syn::ExprCall = syn::parse2(quote! { #path(#tokens) }).unwrap();
                let expr = self.process_html_tag(tag_name, &expr_call);
                *stmt = syn::Stmt::Expr(expr, stmt_macro.semi_token);
            }
        }
        syn::visit_mut::visit_stmt_mut(self, stmt);
    }
}

impl HtmlElementVisitor {
    pub fn get_tag_name(&self, func: &syn::Expr) -> Option<String> {
        let syn::Expr::Path(syn::ExprPath {
            qself: None, path, ..
        }) = func
        else {
            return None;
        };
        self.get_tag_name_from_path(path)
    }

    pub fn get_tag_name_from_path(&self, path: &syn::Path) -> Option<String> {
        if path.segments.len() > 2 {
            return None;
        }
        let mut segments = path.segments.iter();
        let segment = segments.next()?;

        if segment.arguments != syn::PathArguments::None {
            return None;
        }
        let segment = if segment.ident == "html" {
            let segment = segments.next()?;
            if segment.arguments != syn::PathArguments::None {
                return None;
            }
            segment
        } else {
            segment
        };
        if !self.args.html_tags.contains(&segment.ident) {
            return None;
        }
        return Some(segment.ident.to_string());
    }

    pub fn process_html_tag(&mut self, tag_name: String, expr_call: &syn::ExprCall) -> syn::Expr {
        let mut element = XElement {
            tag_name: quote! { #tag_name.into() },
            key: quote! { XKey::default() },
            attributes: vec![],
            events: vec![],
            children: vec![],
            dynamic: None,
            before_render: None,
            after_render: None,
        };
        for arg in &expr_call.args {
            match arg {
                // Attribute
                syn::Expr::Assign(syn::ExprAssign { left, right, .. })
                    if get_attribute_name(left).is_some() =>
                {
                    element.process_attribute(get_attribute_name(left).unwrap(), right);
                }

                // Optional attribute
                syn::Expr::Binary(syn::ExprBinary {
                    left,
                    op: syn::BinOp::BitOrAssign { .. },
                    right,
                    ..
                }) if get_attribute_name(left).is_some() => {
                    element.process_optional_attribute(get_attribute_name(left).unwrap(), right);
                }

                // Dynamic
                syn::Expr::Closure { .. } | syn::Expr::Block { .. } => element.process_dynamic(arg),

                // Children
                syn::Expr::Range(syn::ExprRange {
                    start: Some(children),
                    limits: syn::RangeLimits::HalfOpen { .. },
                    end: None,
                    ..
                }) => element.process_children(children),

                // Child
                _ => element.process_child(self, arg),
            }
        }

        let XElement {
            tag_name,
            key,
            attributes,
            events,
            children,
            dynamic,
            before_render,
            after_render,
        } = element;
        let (generators, value) = match dynamic {
            Some(dynamic) => {
                if attributes.is_empty() && events.is_empty() && children.is_empty() {
                    (quote! {}, dynamic)
                } else {
                    (
                        quote! {
                            compile_error!("Properties of dynamic nodes cannot be defined at the call site");
                        },
                        dynamic,
                    )
                }
            }
            None => {
                let gen_attributes = quote! {
                    let mut gen_attributes = vec![];
                    #(#attributes)*
                };
                let gen_children = quote! {
                    let mut gen_children = vec![];
                    #(#children)*
                };
                let value = quote! {
                    value: XElementValue::Static {
                        attributes: gen_attributes,
                        children: gen_children,
                        events: vec![#(#events),*],
                    }
                };
                let generators = quote! {
                    #gen_attributes
                    #gen_children
                };
                (generators, value)
            }
        };

        let [before_render, after_render] = [before_render, after_render].map(|on_render| {
            let Some(on_render) = on_render else {
                return quote! { None };
            };
            return quote! { Some( OnRenderCallback(Box::new(#on_render)) ) };
        });

        let element = quote! {
            {
                #generators
                XElement {
                    tag_name: #tag_name,
                    key: #key,
                    #value,
                    before_render: #before_render,
                    after_render: #after_render,
                }
            }
        };
        if self.args.debug {
            println!("ELEMENT:\n{element}\n");
        }
        syn::parse2(element).unwrap()
    }
}

fn get_attribute_name(left: &syn::Expr) -> Option<&syn::Ident> {
    let syn::Expr::Path(syn::ExprPath {
        attrs: _,
        qself: None,
        path: syn::Path {
            leading_colon: None,
            segments,
        },
    }) = left
    else {
        return None;
    };
    if segments.len() != 1 {
        return None;
    }
    let syn::PathSegment {
        ident,
        arguments: syn::PathArguments::None,
    } = segments.first().unwrap()
    else {
        return None;
    };
    return Some(ident);
}
