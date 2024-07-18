// This macro is currently disabled because custom attributes are not allowed on statements
// This means we cannot mimic the API of the std allow macro

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{
    parse_macro_input, parse_quote, Attribute, Expr, ExprAsync, ExprAwait, Item, Path, Stmt,
};

/// Allows a runtime lint under an expression or statement.
#[proc_macro_attribute]
pub fn allow(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as AllowInput);

    // Parse the list of variables the user wanted to print.
    let path = parse_macro_input!(args as Path);

    // Use a syntax tree traversal to modify the input in place
    let mut visit_items = VisitItems { path };
    match &mut input {
        AllowInput::Expr(expr) => visit_items.visit_expr_mut(expr),
        AllowInput::Stmt(stmt) => visit_items.visit_stmt_mut(stmt),
        AllowInput::Item(item) => visit_items.visit_item_mut(item),
    }

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote!(#input))
}

enum AllowInput {
    Expr(Expr),
    Stmt(Stmt),
    Item(Item),
}

impl Parse for AllowInput {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Ok(item) = input.fork().parse::<Item>() {
            Ok(AllowInput::Item(item))
        } else if let Ok(stmt) = input.fork().parse::<Stmt>() {
            Ok(AllowInput::Stmt(stmt))
        } else if let Ok(expr) = input.fork().parse::<Expr>() {
            Ok(AllowInput::Expr(expr))
        } else {
            Err(input.error("expected item, statement, or expression"))
        }
    }
}

impl ToTokens for AllowInput {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            AllowInput::Expr(expr) => expr.to_tokens(tokens),
            AllowInput::Stmt(stmt) => stmt.to_tokens(tokens),
            AllowInput::Item(item) => item.to_tokens(tokens),
        }
    }
}
struct VisitItems {
    path: Path,
}

impl VisitMut for VisitItems {
    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        // First check if the expression is an async block
        let mut has_async = HasAsync { has_async: false };
        has_async.visit_expr(i);

        if has_async.has_async {
            // If it is, wrap the entire expression in an async block with allow
            let path = &self.path;
            let new_expr = parse_quote!(async {
                lints::allow_async(#path, async {
                    #i
                })
            });
            *i = new_expr;
        } else {
            // Otherwise, wrap the expression in an allow block
            let path = &self.path;
            let new_expr = parse_quote!(lints::allow(#path, || #i));
            *i = new_expr;
        }
    }

    fn visit_item_mut(&mut self, i: &mut Item) {
        use syn::Item::*;
        // Add the allow macro to the item
        let path = self.path.clone();
        let allow: Attribute = parse_quote!(#[allow(#path)]);

        match i {
            Fn(i) => {
                i.attrs.push(allow);
            }
            Impl(i) => {
                i.attrs.push(allow);
            }
            Mod(i) => {
                i.attrs.push(allow);
            }
            Trait(i) => {
                i.attrs.push(allow);
            }
            _ => {}
        }
    }
}

struct HasAsync {
    has_async: bool,
}

impl<'ast> Visit<'ast> for HasAsync {
    // Don't traverse into items
    fn visit_item(&mut self, _: &Item) {}

    // Don't traverse into async blocks
    fn visit_expr_async(&mut self, _: &ExprAsync) {}

    // If we hit a stmt.await, we must be in an async block
    fn visit_expr_await(&mut self, _: &ExprAwait) {
        self.has_async = true;
    }
}
