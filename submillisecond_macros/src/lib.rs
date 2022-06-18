// mod route;
mod router;

use proc_macro::TokenStream;
// use route::{Route, RouteMethod};
use router::{MethodTries, Router};
use syn::parse_macro_input;

#[proc_macro]
pub fn router(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Router);
    let mut trie_collection = MethodTries::new();
    let expanded = input.expand(&mut trie_collection, None);
    trie_collection.expand();
    expanded.into()
}

// macro_rules! define_route_macro {
//     ($name: ident, $method: ident) => {
//         #[proc_macro_attribute]
//         pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
//             match Route::parse_with_attributes(RouteMethod::$method, attr, item.clone()) {
//                 Ok(route) => route.expand(),
//                 Err(err) => {
//                     TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item])
//                 }
//             }
//         }
//     };
// }

// define_route_macro!(get, GET);
// define_route_macro!(post, POST);
// define_route_macro!(put, PUT);
// define_route_macro!(delete, DELETE);
// define_route_macro!(head, HEAD);
// define_route_macro!(options, OPTIONS);
// define_route_macro!(patch, PATCH);
