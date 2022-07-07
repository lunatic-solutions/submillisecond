mod item_route;
mod item_use_middleware;
pub mod method;
mod router_trie;
mod trie;

use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

use crate::hquote;

use self::{
    item_route::ItemRoute, item_use_middleware::ItemUseMiddleware, method::Method,
    router_trie::RouterTrie,
};

#[derive(Debug)]
pub struct RouterTree {
    pub middleware: Vec<ItemUseMiddleware>,
    pub routes: Vec<ItemRoute>,
}

impl RouterTree {
    pub fn expand(&self) -> TokenStream {
        let trie = RouterTrie::new(self);
        let inner = trie.expand();

        hquote! {
            (|mut req: ::submillisecond::Request,
                mut params: ::submillisecond::params::Params,
                mut reader: ::submillisecond::core::UriReader| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::RouteError> {
                #inner
            }) as ::submillisecond::Router
        }
    }
}

impl Parse for RouterTree {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut middleware = Vec::new();
        while input.peek(Token![use]) {
            middleware.push(input.parse()?);
            let _: Token![;] = input.parse()?;
        }

        let mut routes: Vec<ItemRoute> = Vec::new();
        while Method::peek(input)
            || input.peek(LitStr)
            || (!routes.is_empty() && input.peek(Token![,]))
        {
            routes.push(input.parse()?);
        }

        Ok(RouterTree { middleware, routes })
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use crate::router::tree::RouterTree;

    #[test]
    fn parse_router_tree() {
        let router_tree: RouterTree = parse_quote! {
            use ::a::b::c::{logger};
        };
        println!("{:#?}", router_tree);
    }
}
