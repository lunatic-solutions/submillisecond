
use proc_macro2::TokenStream;
use quote::{quote, TokenStreamExt};

use crate::router::Router;

use super::{
    item_route::{ItemGuard, ItemHandler, ItemRoute},
    item_use_middleware::ItemUseMiddleware,
    method::Method,
    trie::{Node, Trie},
    RouterTree,
};

#[derive(Debug, Default)]
pub struct RouterTrie<'r> {
    // trie to collect subrouters
    subrouters: Trie<TrieValue<'r>>,
    // tries to collect
    get: Trie<TrieValue<'r>>,
    post: Trie<TrieValue<'r>>,
    put: Trie<TrieValue<'r>>,
    delete: Trie<TrieValue<'r>>,
    head: Trie<TrieValue<'r>>,
    options: Trie<TrieValue<'r>>,
    patch: Trie<TrieValue<'r>>,
}

#[derive(Clone, Debug)]
struct TrieValue<'r> {
    guard: &'r Option<ItemGuard>,
    middleware: Vec<&'r ItemUseMiddleware>,
    handler: &'r ItemHandler,
    node_type: NodeType
}

#[derive(Clone, Copy, Debug)]
enum NodeType {
    Handler,
    Subrouter
}

impl<'r> RouterTrie<'r> {
    /// Create a new [`RouterTrie`] from an instance of [`RouterTree`].
    pub fn new(router_tree: &'r RouterTree) -> Self {
        let mut trie = RouterTrie::default();
        trie.collect_tries(None, &router_tree.routes, router_tree.middleware.iter().collect());
        trie
    }

    pub fn expand(&self) -> TokenStream {
        let subrouters_expanded = self.expand_subrouters();
        let handlers_expanded = self.expand_handlers();

        quote! {
            #subrouters_expanded
            #handlers_expanded
        }
    }

    pub fn expand_subrouters(&self) -> TokenStream {
        let mut subrouters_expanded = Self::expand_nodes("", self.subrouters.children());
        if !subrouters_expanded.is_empty() {
            subrouters_expanded.append_all(quote! {
                __reader.reset();
            })
        }
        subrouters_expanded
    }

    pub fn expand_handlers(&self) -> TokenStream {
        let arms = [
            (quote! { ::http::Method::GET }, self.get.children()),
            (quote! { ::http::Method::POST }, self.post.children()),
            (quote! { ::http::Method::PUT }, self.put.children()),
            (quote! { ::http::Method::DELETE }, self.delete.children()),
            (quote! { ::http::Method::HEAD }, self.head.children()),
            (quote! { ::http::Method::OPTIONS }, self.options.children()),
            (quote! { ::http::Method::PATCH }, self.patch.children()),
        ]
        .into_iter()
        .filter_map(|(method, children)| {
            let arms = Self::expand_nodes("", children);
            if arms.is_empty() {
                return None;
            }

            Some(quote! {
                #method => {
                    #arms
                    
                    return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req));
                }
            })
        });

        quote! {
            match *__req.method() {
                #( #arms )*
                _ => {
                    return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req));
                }
            }
        }
    }

    fn expand_nodes(
        full_path: &str,
        nodes: impl Iterator<Item = Node<TrieValue<'r>>>,
    ) -> TokenStream {
        let nodes_expanded = nodes.map(|node| {
            let children = node.children();
            let Node { prefix, value, .. } = node;
            let full_path = format!("{full_path}{prefix}");
            let prefix_len = prefix.len();
            let child_nodes_expanded = Self::expand_nodes(&full_path, children);

            match value {
                Some(TrieValue { guard, middleware, handler, node_type }) => {
                    let ensure_next_slash_expanded = if prefix.len() > 1 && prefix.ends_with('/') {
                        quote! {
                            // since path continues there has to be a separator
                            if !__reader.ensure_next_slash() {
                                return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req));
                            }
                        }
                    } else {
                        quote! {}
                    };

                    let guard_expanded = guard.as_ref().map(|guard| quote! { && #guard });

                    let body = match node_type {
                        NodeType::Handler => Self::expand_handler(handler, &middleware),
                        NodeType::Subrouter => Self::expand_subrouter(handler, &middleware),
                    };

                    quote! {
                        if __reader.peek(#prefix_len) == #prefix #guard_expanded {
                            __reader.read(#prefix_len);
        
                            #body
        
                            #ensure_next_slash_expanded
        
                            #child_nodes_expanded
                        }
                    }
                }
                None if !child_nodes_expanded.is_empty() => quote! {
                    if __reader.peek(#prefix_len) == #prefix {
                        __reader.read(#prefix_len);
                        #child_nodes_expanded
                    }
                },
                None => quote! {}
            }
        });

        quote! {
            #( #nodes_expanded )*
        }
    }

    fn expand_handler(handler: &ItemHandler, middleware: &[&'r ItemUseMiddleware]) -> TokenStream {
        match handler {
            ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                let handler = match handler {
                    ItemHandler::Fn(handler) => quote! { #handler },
                    ItemHandler::Macro(item_macro) => quote! { (#item_macro) },
                    ItemHandler::SubRouter(_) => unreachable!(),
                };

                let middleware_expanded = Self::expand_middleware(middleware);

                quote! {
                    if __reader.is_dangling_slash() {
                        ::submillisecond::Application::merge_extensions(&mut __req, &mut __params);

                        #middleware_expanded

                        return ::std::result::Result::Ok(
                            ::submillisecond::response::IntoResponse::into_response(
                                ::submillisecond::handler::Handler::handle(
                                    #handler
                                        as ::submillisecond::handler::FnPtr<
                                            _,
                                            _,
                                            { ::submillisecond::handler::arity(&#handler) },
                                        >,
                                    __req
                                ),
                            )
                        );
                    }
                }
            },
            ItemHandler::SubRouter(_) => Self::expand_subrouter(handler, middleware),
        }
    }

    fn expand_subrouter(handler: &ItemHandler, middleware: &[&'r ItemUseMiddleware]) -> TokenStream {
        match handler {
            ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                let handler = match handler {
                    ItemHandler::Fn(handler) => quote! { #handler },
                    ItemHandler::Macro(item_macro) => quote! { (#item_macro) },
                    ItemHandler::SubRouter(_) => unreachable!(),
                };

                let middleware_expanded = Self::expand_middleware(middleware);

                quote! {
                    ::submillisecond::Application::merge_extensions(&mut __req, &mut __params);

                    #middleware_expanded

                    return #handler(__req, __params, __reader);
                }
            },
            ItemHandler::SubRouter(subrouter) => {
                let subrouter_expanded = subrouter.expand();

                let middleware_expanded = Self::expand_middleware(middleware);

                quote! {
                    #middleware_expanded

                    (#subrouter_expanded)(__req, __params, __reader)
                }
            },
        }
    }

    fn expand_middleware(middleware: &[&'r ItemUseMiddleware]) -> TokenStream {
        let all_middleware: Vec<_> = middleware.iter().flat_map(|middleware| middleware.tree.items()).collect();

        quote! {
            #(
                ::submillisecond::request_context::inject_middleware(Box::new(<#all_middleware as Default>::default()));
            )*
        }
    }

    /// Insert a subrouter. A subrouter is any handler which is not prefixed with a http method.
    fn insert_subrouter(&mut self, key: String, value: TrieValue<'r>) {
        self.subrouters.insert(key, value);
    }

    /// Insert a handler with a prefixed http method.
    fn insert_handler(&mut self, method: Method, key: String, value: TrieValue<'r>) {
        match method {
            Method::Get(_) => self.get.insert(key, value),
            Method::Post(_) => self.post.insert(key, value),
            Method::Put(_) => self.put.insert(key, value),
            Method::Delete(_) => self.delete.insert(key, value),
            Method::Head(_) => self.head.insert(key, value),
            Method::Options(_) => self.options.insert(key, value),
            Method::Patch(_) => self.patch.insert(key, value),
        };
    }

    /// Recursively collect handlers and subrouters
    fn collect_tries(&mut self, prefix: Option<String>, routes: &'r [ItemRoute], all_middleware: Vec<&'r ItemUseMiddleware>) {
        for ItemRoute {
            method,
            path,
            guard,
            middleware,
            handler,
            ..
        } in routes.iter()
        {
            let new_path = prefix
                .as_ref()
                .map(|prefix| format!("{prefix}{}", path.value()))
                .unwrap_or_else(|| path.value());

            let mut all_middleware = all_middleware.clone();
            if let Some(middleware) = middleware {
                all_middleware.push(middleware);
            }

            match handler {
                ItemHandler::SubRouter(Router::Tree(tree)) => {
                    self.collect_tries(Some(new_path), &tree.routes, all_middleware);
                }
                _ => {
                    let value = TrieValue {
                        guard,
                        middleware: all_middleware,
                        handler,
                        node_type: if method.is_some() { NodeType::Handler } else { NodeType::Subrouter },
                    };

                    if let Some(method) = *method {
                        self.insert_handler(method, new_path, value);
                    } else {
                        self.insert_subrouter(new_path, value);
                    }
                }
            }
        }
    }
}
