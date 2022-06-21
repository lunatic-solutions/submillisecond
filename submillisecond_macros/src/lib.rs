mod router;
mod static_dir;

use proc_macro::TokenStream;
use router::Router;
use static_dir::StaticDir;
use syn::parse_macro_input;

#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Router);
    input.expand().into()
}

#[proc_macro]
pub fn static_dir(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as StaticDir);
    input.expand().into()
}
