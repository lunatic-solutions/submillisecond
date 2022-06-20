mod list;
mod tree;

use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

use self::{
    list::RouterList,
    tree::{method::Method, RouterTree},
};
pub use tree::MethodTries;

#[derive(Debug)]
pub enum Router {
    List(RouterList), // [a, b, c]
    Tree(RouterTree), // { "/" => ... }
}

// impl Router {
//     pub fn expand(&self, router: &mut MethodTries, prefix: Option<&LitStr>) -> TokenStream {
//         match self {
//             Router::List(router_list) => router_list.expand(),
//             Router::Tree(router_tree) => router_tree.expand(router, prefix),
//         }
//     }
// }

impl Parse for Router {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Router::List(RouterList::default()));
        }

        if input.peek(LitStr) || Method::peek(input) || input.peek(Token![use]) {
            return Ok(Router::Tree(input.parse()?));
        }

        Ok(Router::List(input.parse()?))
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::Router;

    #[test]
    fn parse_router() {
        let _: Router = parse_quote! {foo};
    }
}
