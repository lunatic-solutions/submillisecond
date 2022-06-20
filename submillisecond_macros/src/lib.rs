mod router;

use proc_macro::TokenStream;
use router::Router;
use syn::parse_macro_input;

#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Router);
    input.expand().into()
}
