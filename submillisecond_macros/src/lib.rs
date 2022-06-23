mod router;
mod trie;

use proc_macro::TokenStream;
use router::{MethodTries, Router};
use syn::parse_macro_input;

#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    match parse_macro_input!(input as Router) {
        Router::Tree(tree) => MethodTries::new().expand(tree).into(),
        Router::List(_) => panic!("Cannot parse RouterList"),
    }
}
