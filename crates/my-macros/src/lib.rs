use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn simple_main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // 1. Parse the incoming `async fn main() { ... }` into a syntax tree structure
    let input_fn = parse_macro_input!(item as ItemFn);

    // Extract the function's body block `{ ... }` and function name `main`
    let fn_name = &input_fn.sig.ident;
    let fn_body = &input_fn.block;

    // 2. Construct new Rust code using `quote!` (replaces #vars with extracted parts)
    let expanded = quote! {
        fn #fn_name() {
            println!(">>> [Macro] Initializing Tokio runtime...");
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async #fn_body);
            println!(">>> [Macro] Runtime shut down cleanly.");
        }
    };

    // 3. Hand the generated code back to the compiler
    TokenStream::from(expanded)
}