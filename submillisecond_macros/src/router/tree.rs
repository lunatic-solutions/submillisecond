mod item_route;
mod item_use_middleware;
pub mod method;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use submillisecond_core::router::tree::{Node, NodeType};
use syn::{
    parse::{Parse, ParseStream},
    Expr, Index, LitStr, Token,
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
        let middleware = self.middleware();
        let middleware_before = {
            let invoke_middleware = middleware.iter().map(|item| {
                quote! {
                    <#item as ::submillisecond::Middleware>::before(&mut __req)
                }
            });

            quote! {
                 let __middleware_calls = ( #( #invoke_middleware, )* );
            }
        };
        let middleware_after = (0..middleware.len()).map(|idx| {
            let idx = Index::from(idx);
            quote! {
                __middleware_calls.#idx.after(__resp);
            }
        });

        let mut router_node = Node::default();
        for route in &self.routes {
            let mut path = route.path.value();
            if route.method.is_none() {
                if path == "/" {
                    path.push_str("*__slug");
                } else {
                    path.push_str("/*__slug");
                }
            }
            if let Err(err) = router_node.insert(route.path.value(), (route.method, &route.path)) {
                return syn::Error::new(route.path.span(), err.to_string()).into_compile_error();
            }
            if let Err(err) = router_node.insert(path, (route.method, &route.path)) {
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
                        Method::Get(get) => quote! { ::submillisecond::http::Method::#get },
                        Method::Post(post) => quote! { ::submillisecond::http::Method::#post },
                        Method::Put(put) => quote! { ::submillisecond::http::Method::#put },
                        Method::Delete(delete) => {
                            quote! { ::submillisecond::http::Method::#delete }
                        }
                        Method::Head(head) => quote! { ::submillisecond::http::Method::#head },
                        Method::Options(options) => {
                            quote! { ::submillisecond::http::Method::#options }
                        }
                        Method::Patch(patch) => {
                            quote! { ::submillisecond::http::Method::#patch }
                        }
                    })
                    .map(
                        |method| quote! { __method.as_ref().map(|method| method == #method).unwrap_or(false) },
                    )
                    .unwrap_or_else(|| quote! { true });

                let guards_expanded = guard
                    .as_ref()
                    .map(|guard| &*guard.guard)
                    .map(expand_guard_struct)
                    .map(|guard| quote! { && { #guard } });

                let (middleware_before, middleware_after) = if let Some(m) = middleware {
                    let items = m.tree.items();
                    let invoke_middleware = items
                        .iter()
                        .map(|item| {
                            quote! {
                                <#item as ::submillisecond::Middleware>::before(&mut __req)
                            }
                        });

                    let before_calls = quote! {
                        let __middleware_calls = ( #( #invoke_middleware, )* );
                    };

                    let after_calls = (0..items.len())
                        .map(|idx| {
                            let idx = Index::from(idx);
                            quote! {
                                ::submillisecond::Middleware::after(__middleware_calls.#idx, &mut __resp);
                            }
                        });

                    (before_calls, quote! {#( #after_calls )*})
                } else {
                    (quote! {}, quote! {})
                };

                match handler {
                    ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                        let handler = match handler {
                            ItemHandler::Fn(handler_fn) => quote! { #handler_fn },
                            ItemHandler::Macro(item_macro) => quote! { ( #item_macro ) },
                            ItemHandler::SubRouter(_) => unreachable!(),
                        };

                        quote! {
                            #path if #method_expanded #guards_expanded => {
                                #middleware_before

                                let mut __resp = ::submillisecond::response::IntoResponse::into_response(
                                    ::submillisecond::handler::Handler::handle(
                                        #handler
                                            as ::submillisecond::handler::FnPtr<
                                                _,
                                                _,
                                                { ::submillisecond::handler::arity(&#handler) },
                                            >,
                                        __req,
                                    ),
                                );

                                #middleware_after

                                return ::std::result::Result::Ok(__resp);
                            }
                        }
                    },
                    ItemHandler::SubRouter(sub_router) => {
                        let sub_router_expanded = sub_router.expand();

                        quote! {
                            #path if #method_expanded #guards_expanded => {
                                const SUB_ROUTER: ::submillisecond::handler::HandlerFn = #sub_router_expanded;
                                return SUB_ROUTER(__req);
                            }
                        }
                    },
                }
            },
        );

        quote! {
            (|mut __req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                const __ROUTER: ::submillisecond_core::router::Router<'static, (::std::option::Option<::submillisecond::http::Method>, &'static str)> = ::submillisecond_core::router::Router::from_node(
                    #node_expanded,
                );

                let __path = &__req
                    .extensions()
                    .get::<::submillisecond::router::Route>()
                    .unwrap()
                    .0;
                let __route_match = __ROUTER.at(__path);

                #middleware_before

                let mut __resp = match __route_match {
                    ::std::result::Result::Ok(::submillisecond_core::router::Match {
                        params: __params,
                        values: __values,
                    }) => {
                        if !__params.is_empty() {
                            if let Some(slug) = __params.get("__slug") {
                                let mut path = ::std::string::String::with_capacity(slug.len() + 1);
                                path.push('/');
                                path.push_str(slug);

                                __req
                                    .extensions_mut()
                                    .insert(::submillisecond::router::Route(::std::borrow::Cow::Owned(path)));
                            }

                            match __req
                                .extensions_mut()
                                .get_mut::<::submillisecond_core::router::params::Params>()
                            {
                                ::std::option::Option::Some(params_ext) => params_ext.merge(__params),
                                ::std::option::Option::None => {
                                    __req.extensions_mut().insert(__params);
                                }
                            }
                        } else {
                            __req
                                .extensions_mut()
                                .insert(::submillisecond::router::Route(::std::borrow::Cow::Borrowed("/")));
                        }

                        (move || {
                            for (__method, __route) in __values {
                                match *__route {
                                    #( #arms_expanded, )*
                                    _ => {},
                                }
                            }

                            ::std::result::Result::Err(
                                ::submillisecond::router::RouteError::RouteNotMatch(__req),
                            )
                        })()
                    }
                    ::std::result::Result::Err(_) => {
                        ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req))
                    }
                };

                if let Ok(ref mut __resp) = &mut __resp {
                    #( #middleware_after )*
                }

                __resp
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

    let value_expanded = value.iter().map(|(method, route)| {
        let method_expanded = method
            .as_ref()
            .map(|method| match method {
                Method::Get(get) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#get) }
                }
                Method::Post(post) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#post) }
                }
                Method::Put(put) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#put) }
                }
                Method::Delete(delete) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#delete) }
                }
                Method::Head(head) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#head) }
                }
                Method::Options(options) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#options) }
                }
                Method::Patch(patch) => {
                    quote! { ::std::option::Option::Some(::submillisecond::http::Method::#patch) }
                }
            })
            .unwrap_or_else(|| quote! { ::std::option::Option::None });

        quote! { (#method_expanded, #route) }
    });

    quote! {
        ::submillisecond_core::router::tree::ConstNode {
            priority: #priority,
            wild_child: #wild_child,
            indices: &[#( #indices_expanded, )*],
            node_type: #node_type_expanded,
            value: &[#( #value_expanded ),*],
            prefix: &[#( #prefix_expanded, )*],
            children: &[#( #children_expanded, )*],
        }
    }
}

fn expand_guard_struct(guard: &Expr) -> TokenStream {
    match guard {
        Expr::Binary(expr_binary) => {
            let left = expand_guard_struct(&expr_binary.left);
            let op = &expr_binary.op;
            let right = expand_guard_struct(&expr_binary.right);

            quote! {
                #left #op #right
            }
        }
        Expr::Paren(expr_paren) => {
            let expr = expand_guard_struct(&expr_paren.expr);
            quote_spanned!(expr_paren.paren_token.span=> (#expr))
        }
        expr => quote! {
            ::submillisecond::guard::Guard::check(&#expr, &__req)
        },
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
