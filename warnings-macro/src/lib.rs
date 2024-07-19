use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{parse_macro_input, FnArg, ItemFn};

/// Turns a function into a warning that is only called if the lint is enabled.
#[proc_macro_attribute]
pub fn warning(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let fn_name = &input.sig.ident;

    let argument_types = input
        .sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(arg) => Some(&arg.ty),
        })
        .collect::<Vec<_>>();
    let argument_idents = input
        .sig
        .inputs
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(arg) => Some(syn::Ident::new(&format!("arg{}", index), arg.pat.span())),
        })
        .collect::<Vec<_>>();

    let private_mod = format_ident!("__{}", fn_name);

    let vis = &input.vis;

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote! {
        #[allow(non_camel_case_types)]
        #vis struct #fn_name {}

        mod #private_mod {
            use super::*;

            pub(crate) enum __Callable {
                #[allow(non_camel_case_types)]
                #fn_name,
            }

            impl std::ops::Deref for __Callable {
                type Target = fn(#(#argument_types),*);
                fn deref(&self) -> &Self::Target {
                    fn __run_if_enabled(#(#argument_idents: #argument_types),*) {
                        #fn_name::ID.if_enabled(|| {
                            #input
                            #fn_name(#(#argument_idents),*);
                        });
                    }
                    &(__run_if_enabled as fn(#(#argument_types),*))
                }
            }
        }
        #vis use #private_mod::__Callable::*;

        impl warnings::Warning for #fn_name {
            const ID: warnings::WarningId = warnings::WarningId::new(&#fn_name);
        }
    })
}
