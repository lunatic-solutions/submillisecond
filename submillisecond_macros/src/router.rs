pub use item_catch_all::*;
pub use item_route::*;
pub use item_with_middleware::*;
pub use method::*;
pub use router_trie::*;
pub use trie::*;

mod item_catch_all;
mod item_route;
mod item_with_middleware;
mod method;
mod router_trie;
mod trie;

use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Token};

use crate::hquote;

#[derive(Clone, Debug)]
pub struct Router {
    pub middleware: Option<ItemWithMiddleware>,
    pub routes: Vec<ItemRoute>,
    pub catch_all: Option<ItemCatchAll>,
}

impl Router {
    pub fn expand(&self) -> TokenStream {
        let trie = RouterTrie::new(self);
        let inner = trie.expand();

        hquote! {
            (|mut req: ::submillisecond::RequestContext| -> ::submillisecond::response::Response {
                #inner
            }) as ::submillisecond::Router
        }
    }
}

impl Parse for Router {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let middleware = if input.peek(with) {
            let middleware = input.parse()?;
            let _: Token![;] = input.parse()?;
            Some(middleware)
        } else {
            None
        };

        let mut routes: Vec<ItemRoute> = Vec::new();
        while Method::peek(input)
            || input.peek(LitStr)
            || (!routes.is_empty() && input.peek(Token![,]))
        {
            routes.push(input.parse()?);
        }

        let catch_all = input.peek(Token![_]).then(|| input.parse()).transpose()?;

        Ok(Router {
            middleware,
            routes,
            catch_all,
        })
    }
}
