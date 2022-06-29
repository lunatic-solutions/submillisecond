mod item_route;
mod item_use_middleware;
pub mod method;
mod method_tries;
mod trie;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

use self::{
    item_route::ItemRoute, item_use_middleware::ItemUseMiddleware, method::Method,
    method_tries::MethodTries,
};

#[derive(Debug)]
pub struct RouterTree {
    pub middleware: Vec<ItemUseMiddleware>,
    pub routes: Vec<ItemRoute>,
}

impl RouterTree {
    pub fn expand(&self) -> TokenStream {
        let method_tries_expanded = MethodTries::new(self).expand();

        quote! {
            (|mut __req: ::submillisecond::Request,
                mut __params: ::submillisecond::params::Params,
                mut __reader: ::submillisecond::core::UriReader| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                #method_tries_expanded
            }) as ::submillisecond::handler::HandlerFn
        }
    }

    /// Returns all the use middleware items with their full path.
    fn middleware(&self) -> Vec<TokenStream> {
        self.middleware.iter().fold(
            Vec::with_capacity(self.middleware.len()),
            |mut acc, item| {
                let items = item.tree.items();
                match item.leading_colon {
                    Some(leading_colon) => {
                        acc.extend(
                            items
                                .into_iter()
                                .map(|item| quote! { #leading_colon #item }),
                        );
                    }
                    None => acc.extend(items.into_iter()),
                }
                acc
            },
        )
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
