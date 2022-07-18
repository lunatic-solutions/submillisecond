pub mod method;

mod item_catch_all;
mod item_route;
mod item_use_middleware;
mod router_trie;
mod trie;

use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

use crate::hquote;

use self::{
    item_catch_all::ItemCatchAll, item_route::ItemRoute, item_use_middleware::ItemUseMiddleware,
    method::Method, router_trie::RouterTrie,
};

#[derive(Clone, Debug)]
pub struct RouterTree {
    pub middleware: Vec<ItemUseMiddleware>,
    pub routes: Vec<ItemRoute>,
    pub catch_all: Option<ItemCatchAll>,
}

impl RouterTree {
    pub fn expand(&self) -> TokenStream {
        let trie = RouterTrie::new(self);
        let inner = trie.expand();

        hquote! {
            (|mut req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::RouteError> {
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

        let catch_all = input.peek(Token![_]).then(|| input.parse()).transpose()?;

        Ok(RouterTree {
            middleware,
            routes,
            catch_all,
        })
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
