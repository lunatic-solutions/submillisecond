mod router;

use proc_macro::TokenStream;
// use route::{Route, RouteMethod};
use router::{MethodTries, Router};
use syn::parse_macro_input;

#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    if let Router::Tree(tree) = parse_macro_input!(input as Router) {
        return MethodTries::new().expand(tree).into();
    }
    panic!("Failed to parse a RouterTree");
}
