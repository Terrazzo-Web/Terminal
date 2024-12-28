//! A simple macro to cloning variable before passing them into a `move` closure or async block.
//!
//! # Usage
//! The list of variables to clone are defined at the beginning of the block,
//! which is easier to add cloning for variables.
//!
//! With autoclone:
//! ```
//! use autoclone_macro::autoclone;
//! #[autoclone]
//! fn test() {
//!     let my_string = "Hello, World!".to_string();
//!     let callback = move || {
//!         // Declare variables that need cloning.
//!         // `autoclone!(<my_variabke>, <other_variabke>, ...);`
//!         // Just remove the `autoclone!(...);` statement if cloning is not required.
//!         autoclone!(my_string);
//!         println!("Inside the move callback: {my_string}");
//!     };
//!     println!("Outside the move callback: {my_string}");
//!     callback();
//! }
//! test();
//! ```
//!
//! # Comparison with clone-macro
//! With clone-macro:
//! ```
//! fn test() {
//!     use clone_macro::clone;
//!     let my_string = "Hello, World!".to_string();
//!     // Adding cloning is not trivial
//!     // - requires adding/removing `clone!([my_string]` if cloning is necessary
//!     // - requires adding/removing the corresponding closing `)`
//!     let callback = clone!([my_string], move || {
//!         println!("Inside the move callback: {my_string}");
//!     });
//!     println!("Outside the move callback: {my_string}");
//!     callback();
//! }
//! test();
//! ```
//!
//! See also https://docs.rs/clone-macro
//!
//! # Syntax sugar
//! The `autoclone!()` macro does not exist.
//! Instead, the `#[autoclone]` proc macro modifies the code.
//! You can see the modified code using `#[autoclone(debug = true)]`
//!
//! The previous example expands to
//! ```
//! fn test() {
//!     let my_string = "Hello, World!".to_string();
//!     let callback = {
//!         let my_string = my_string.to_owned();
//!         move || {
//!             println!("Inside the move callback: {my_string}");
//!         }
//!     };
//!     println!("Outside the move callback: {my_string}");
//!     callback();
//! }
//! ```

use darling::ast::NestedMeta;
use darling::FromMeta;
use quote::format_ident;
use quote::quote;
use quote::ToTokens as _;
use syn::visit_mut;
use syn::visit_mut::VisitMut;
use syn::Block;
use syn::Macro;
use syn::Stmt;
use syn::StmtMacro;
use syn::Token;

mod tests;

/// A simple macro to cloning variable before passing them into a `move` closure or async block.
///
/// See [crate] documentations for details.
#[proc_macro_attribute]
pub fn autoclone(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    autoclone2(attr.into(), item.into()).into()
}

fn autoclone2(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(attr) {
        Ok(args) => args,
        Err(error) => return darling::Error::from(error).write_errors(),
    };
    let attr_args = match AutoCloneArgs::from_list(&attr_args) {
        Ok(args) => args,
        Err(error) => return error.write_errors(),
    };

    let mut item: syn::Item = match syn::parse2(item) {
        Ok(item) => item,
        Err(err) => return err.into_compile_error(),
    };
    if attr_args.debug {
        println!("\nBefore:\n{}\n", item_to_string(&item));
    }
    let mut callback_visitor = CallbackVisitor::default();
    callback_visitor.visit_item_mut(&mut item);
    if attr_args.debug {
        println!("\nAfter:\n{}\n", item_to_string(&item));
    }
    if !attr_args.allow_unused && callback_visitor.count == 0 {
        return quote! { compile_error!("autoclone is not used") };
    }
    return item.to_token_stream();
}

#[derive(Debug, FromMeta)]
struct AutoCloneArgs {
    #[darling(default)]
    debug: bool,

    #[darling(default)]
    allow_unused: bool,
}

#[derive(Default)]
struct CallbackVisitor {
    count: i32,
}

impl VisitMut for CallbackVisitor {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        visit_mut::visit_expr_mut(self, expr);
        let mut cloned_variables = ClonedVariablesVisitor::default();
        match &expr {
            syn::Expr::Async(_) | syn::Expr::Closure(_) => (),
            _ => return,
        };
        cloned_variables.visit_expr_mut(expr);
        if cloned_variables.to_clone.is_empty() {
            return;
        }
        let to_clone = cloned_variables.to_clone;
        let to_clone = quote! {
            #( let #to_clone = #to_clone.to_owned(); )*
        };
        let owned_closure = quote! {
            {
                #to_clone
                #expr
            }
        };
        self.count += 1;
        *expr = syn::parse2(owned_closure).expect("The new closure");
    }
}

#[derive(Default)]
struct ClonedVariablesVisitor {
    to_clone: Vec<syn::Ident>,
}

impl VisitMut for ClonedVariablesVisitor {
    fn visit_block_mut(&mut self, block: &mut Block) {
        visit_mut::visit_block_mut(self, block);
        let mut i = 0;
        while i < block.stmts.len() {
            let stmt = &mut block.stmts[i];
            let Stmt::Macro(StmtMacro { mac, .. }) = stmt else {
                break;
            };
            if !self.process_macro(mac) {
                break;
            }
            i += 1;
        }
        block.stmts.drain(0..i);
    }
}

impl ClonedVariablesVisitor {
    fn process_macro(&mut self, mac: &mut Macro) -> bool {
        if mac.path.segments.len() != 1 {
            return false;
        }
        let path_segment = mac.path.segments.first().expect("Non-empty path segments");
        let syn::PathArguments::None = path_segment.arguments else {
            return false;
        };
        if path_segment.ident != format_ident!("autoclone") {
            return false;
        }
        type IdentifierList = syn::punctuated::Punctuated<syn::Ident, Token![,]>;
        let idents: IdentifierList = mac
            .parse_body_with(IdentifierList::parse_terminated)
            .unwrap();
        self.to_clone.extend(idents);
        return true;
    }
}

fn item_to_string(item: &syn::Item) -> String {
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: vec![],
        items: vec![item.clone()],
    })
}

#[cfg(test)]
use clone_macro as _;
