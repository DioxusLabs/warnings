use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Turns a function into a warning that is only called if the lint is enabled.
#[proc_macro_attribute]
pub fn warning(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let fn_name = &input.sig.ident;

    // Hand the resulting function body back to the compiler.
    TokenStream::from(quote! {
        #[allow(non_camel_case_types)]
        pub struct #fn_name;

        struct private;

        impl std::ops::Deref for private {
            type Target = fn();
            fn deref(&self) -> &Self::Target {
                fn __run_if_enabled() {
                    #fn_name::WARNING.if_enabled(|| {
                        #input
                        #fn_name();
                    });
                }
                &(__run_if_enabled as fn())
            }
        }

        impl #fn_name {
            const WARNING: Warning = Warning::new(&#fn_name);
        }

        impl std::ops::Deref for #fn_name {
            type Target = fn();
            fn deref(&self) -> &Self::Target {
                &private
            }
        }
    })
}
