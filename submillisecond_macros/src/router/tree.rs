mod item_route;
mod item_use_middleware;
pub mod method;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use submillisecond_core::router::tree::{Node, NodeType};
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token,
};

use self::{
    item_route::{ItemHandler, ItemRoute},
    item_use_middleware::ItemUseMiddleware,
    method::Method,
};

#[derive(Debug)]
pub struct RouterTree {
    middleware: Vec<ItemUseMiddleware>,
    routes: Vec<ItemRoute>,
}

impl RouterTree {
    pub fn expand(&self) -> TokenStream {
        // let middleware_expanded = self.middleware().into_iter().map(|middleware| {});

        let mut router_node = Node::default();
        for route in &self.routes {
            if let Err(err) = router_node.insert(route.path.value(), (route.method, &route.path)) {
                return syn::Error::new(route.path.span(), err.to_string()).into_compile_error();
            }
        }

        let node_expanded = expand_node(&router_node);

        let arms_expanded = self.routes.iter().map(
            |ItemRoute {
                 method,
                 path,
                 guard,
                 middleware,
                 handler,
                 ..
             }| {
                let method_expanded = method
                    .as_ref()
                    .map(|method| match method {
                        Method::Get(get) => quote! { ::http::Method::#get },
                        Method::Post(post) => quote! { ::http::Method::#post },
                        Method::Put(put) => quote! { ::http::Method::#put },
                        Method::Delete(delete) => {
                            quote! { ::http::Method::#delete }
                        }
                        Method::Head(head) => quote! { ::http::Method::#head },
                        Method::Options(options) => {
                            quote! { ::http::Method::#options }
                        }
                        Method::Patch(patch) => {
                            quote! { ::http::Method::#patch }
                        }
                    })
                    .map(
                        |method| quote! { method.map(|method| method == #method).unwrap_or(false) },
                    )
                    .unwrap_or_else(|| quote! { true });

                let guards_expanded = guard
                    .as_ref()
                    .map(|guard| &*guard.guard)
                    .map(|guard| quote! { && { #guard } });

                let handler_ident = match handler {
                    ItemHandler::Fn(f) => f.to_token_stream(),
                    ItemHandler::SubRouter(_) => todo!(),
                };

                quote! {
                    #path if #method_expanded #guards_expanded => ::std::result::Result::Ok(
                        ::submillisecond::response::IntoResponse::into_response(
                            ::submillisecond::handler::Handler::handle(
                                #handler_ident
                                    as ::submillisecond::handler::FnPtr<
                                        _,
                                        _,
                                        { ::submillisecond::handler::arity(&#handler_ident) },
                                    >,
                                req,
                            ),
                        ),
                    )
                }
            },
        );

        quote! {
            ((|mut req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                const ROUTER: ::submillisecond_core::router::Router<'static, &'static str> = ::submillisecond_core::router::Router::from_node(
                    #node_expanded,
                );

                let path = req
                    .extensions()
                    .get::<::submillisecond::router::Route>()
                    .unwrap()
                    .path();
                let route_match = ROUTER.at(path);
                match route_match {
                    ::std::result::Result::Ok(::submillisecond_core::router::Match {
                        params,
                        value: (method, route),
                    }) => {
                        if !params.is_empty() {
                            match req
                                .extensions_mut()
                                .get_mut::<::submillisecond_core::router::params::Params>()
                            {
                                ::std::option::Option::Some(params_ext) => params_ext.merge(params),
                                ::std::option::Option::None => {
                                    req.extensions_mut().insert(params);
                                }
                            }
                        }

                        match *route {
                            #( #arms_expanded, )*
                            _ => ::std::result::Result::Err(
                                ::submillisecond::router::RouteError::RouteNotMatch(req),
                            ),
                        }
                    }
                    ::std::result::Result::Err(_) => {
                        ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(req))
                    }
                }
            }) as ::submillisecond::router::HandlerFn)
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

fn expand_node(
    Node {
        priority,
        wild_child,
        indices,
        node_type,
        value,
        prefix,
        children,
    }: &Node<(Option<Method>, &LitStr)>,
) -> proc_macro2::TokenStream {
    let indices_expanded = indices.iter().map(|indicie| {
        quote! {
            #indicie
        }
    });

    let node_type_expanded = match node_type {
        NodeType::Root => quote! { ::submillisecond_core::router::tree::NodeType::Root },
        NodeType::Param => quote! { ::submillisecond_core::router::tree::NodeType::Param },
        NodeType::CatchAll => quote! { ::submillisecond_core::router::tree::NodeType::CatchAll },
        NodeType::Static => quote! { ::submillisecond_core::router::tree::NodeType::Static },
    };

    let prefix_expanded = prefix.iter().map(|prefix| {
        quote! {
            #prefix
        }
    });

    let children_expanded = children.iter().map(expand_node);

    let value_expanded = unsafe { value.as_ref().map(|value| &*value.get()) }
        .map(|(method, route)| {
            let method_expanded = method.as_ref().map(|method| match method {
                Method::Get(get) => quote! { ::std::result::Option::Some(::http::Method::#get) },
                Method::Post(post) => quote! { ::std::result::Option::Some(::http::Method::#post) },
                Method::Put(put) => quote! { ::std::result::Option::Some(::http::Method::#put) },
                Method::Delete(delete) => {
                    quote! { ::std::result::Option::Some(::http::Method::#delete) }
                }
                Method::Head(head) => quote! { ::std::result::Option::Some(::http::Method::#head) },
                Method::Options(options) => {
                    quote! { ::std::result::Option::Some(::http::Method::#options) }
                }
                Method::Patch(patch) => {
                    quote! { ::std::result::Option::Some(::http::Method::#patch) }
                }
            });

            quote! { Some((#method_expanded, #route)) }
        })
        .unwrap_or_else(|| quote! { None });

    quote! {
        ::submillisecond_core::router::tree::ConstNode {
            priority: #priority,
            wild_child: #wild_child,
            indices: &[#( #indices_expanded, )*],
            node_type: #node_type_expanded,
            value: #value_expanded,
            prefix: &[#( #prefix_expanded, )*],
            children: &[#( #children_expanded, )*],
        }
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
