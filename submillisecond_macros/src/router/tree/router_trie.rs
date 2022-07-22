use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::TokenStreamExt;
use regex::Regex;

use crate::{hquote, router::Router};

use super::{
    item_catch_all::ItemCatchAll,
    item_route::{ItemGuard, ItemHandler, ItemRoute},
    item_use_middleware::ItemUseMiddleware,
    method::Method,
    trie::{Node, Trie},
    RouterTree,
};

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?P<lit_prefix>[^:]*):(?P<param>[a-zA-Z_]+)(?P<lit_suffix>.*)").unwrap();
}

#[derive(Debug, Default)]
pub struct RouterTrie<'r> {
    catch_all: Option<&'r ItemCatchAll>,
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
    guards: Vec<&'r ItemGuard>,
    handler: &'r ItemHandler,
    method: &'r Option<Method>,
    middleware: Vec<&'r ItemUseMiddleware>,
    node_type: NodeType,
}

#[derive(Clone, Copy, Debug)]
enum NodeType {
    Handler,
    Subrouter,
}

#[derive(Clone, Debug)]
struct ExpandedNodeParts {
    guards_expanded: TokenStream,
    handler_expanded: TokenStream,
}

macro_rules! quote_reader_fallback {($($tt:tt)*) => {{
    let mut _s = quote::__private::TokenStream::new();
    let _span = proc_macro2::Span::mixed_site();
    quote::quote_each_token_spanned!{ _s _span $($tt)* }
    hquote! {
        let cursor = req.reader.cursor;
        {#_s}
        #[allow(unreachable_code)]
        { req.reader.cursor = cursor; }
    }
}}}

impl<'r> RouterTrie<'r> {
    /// Create a new [`RouterTrie`] from an instance of [`RouterTree`].
    pub fn new(router_tree: &'r RouterTree) -> Self {
        let mut trie = RouterTrie {
            catch_all: router_tree.catch_all.as_ref(),
            ..Default::default()
        };
        trie.collect_tries(
            None,
            &router_tree.routes,
            router_tree.middleware.iter().collect(),
            Vec::new(),
            &router_tree.catch_all,
        );
        trie
    }

    /// Expand function body.
    pub fn expand(&self) -> TokenStream {
        let subrouters_expanded = self.expand_subrouters();
        let handlers_expanded = self.expand_handlers();

        hquote! {
            #subrouters_expanded
            #handlers_expanded
        }
    }

    /// Expand subrouters.
    fn expand_subrouters(&self) -> TokenStream {
        let mut subrouters_expanded = self.expand_nodes("", self.subrouters.children());
        if !subrouters_expanded.is_empty() {
            subrouters_expanded.append_all(hquote! {
                req.reader.reset();
            })
        }
        subrouters_expanded
    }

    /// Expand handlers for each http method as a match statement.
    fn expand_handlers(&self) -> TokenStream {
        let catch_all_expanded = self.expand_catch_all();

        let arms = [
            (hquote! { ::http::Method::GET }, self.get.children()),
            (hquote! { ::http::Method::POST }, self.post.children()),
            (hquote! { ::http::Method::PUT }, self.put.children()),
            (hquote! { ::http::Method::DELETE }, self.delete.children()),
            (hquote! { ::http::Method::HEAD }, self.head.children()),
            (hquote! { ::http::Method::OPTIONS }, self.options.children()),
            (hquote! { ::http::Method::PATCH }, self.patch.children()),
        ]
        .into_iter()
        .filter_map(|(method, children)| {
            let arms = self.expand_nodes("", children);
            if arms.is_empty() {
                return None;
            }

            Some(hquote! {
                #method => {
                    #arms
                    #catch_all_expanded
                }
            })
        });

        hquote! {
            match *req.method() {
                #( #arms )*
                _ => {
                    #catch_all_expanded
                }
            }
        }
    }

    /// Expand an iterator of nodes, typically [`super::trie::Children`].
    fn expand_nodes(
        &self,
        full_path: &str,
        nodes: impl Iterator<Item = Node<TrieValue<'r>>>,
    ) -> TokenStream {
        let nodes_expanded = nodes.map(|node| self.expand_node(full_path, &node));

        hquote! {
            #( #nodes_expanded )*
        }
    }

    /// Expand a single node.
    fn expand_node(&self, full_path: &str, node: &Node<TrieValue<'r>>) -> TokenStream {
        let children = node.children();
        let Node { prefix, value, .. } = &node;
        let full_path = format!("{full_path}{prefix}");
        let captures = Self::capture_param_parts(prefix);
        let child_nodes_expanded = self.expand_nodes(&full_path, children);
        let prefix_len = prefix.len();

        match value {
            Some(value) => match captures {
                Some((prefix, param, suffix)) => {
                    self.expand_param_node(&full_path, node, prefix, param, suffix)
                }
                None => self.expand_static_node(prefix, value, child_nodes_expanded),
            },
            None if !child_nodes_expanded.is_empty() => quote_reader_fallback! {
                if req.reader.peek(#prefix_len) == #prefix {
                    req.reader.read(#prefix_len);
                    #child_nodes_expanded
                }
            },
            None => hquote! {},
        }
    }

    /// Expand a static node with no parameters.
    fn expand_static_node(
        &self,
        prefix: &str,
        value: &TrieValue<'r>,
        child_nodes_expanded: TokenStream,
    ) -> TokenStream {
        let prefix_len = prefix.len();

        let child_nodes_expanded = if prefix.len() > 1 && prefix.ends_with('/') {
            hquote! {
                // since path continues there has to be a separator
                if req.reader.ensure_next_slash() {
                    #child_nodes_expanded
                }
            }
        } else {
            child_nodes_expanded
        };

        let ExpandedNodeParts {
            guards_expanded,
            handler_expanded,
        } = Self::expand_node_parts(prefix, value);

        quote_reader_fallback! {
            if req.reader.peek(#prefix_len) == #prefix #guards_expanded {
                req.reader.read(#prefix_len);

                #child_nodes_expanded

                #handler_expanded
            }
        }
    }

    /// Expand a node with parameter(s) recursively.
    fn expand_param_node(
        &self,
        full_path: &str,
        node: &Node<TrieValue<'r>>,
        prefix: &str,
        param: &str,
        suffix: &str,
    ) -> TokenStream {
        let suffix_len = suffix.len();
        let mut expanded = hquote! {};

        match suffix {
            "" | "/" => {
                if let Some(value) = &node.value {
                    let ExpandedNodeParts {
                        guards_expanded,
                        handler_expanded,
                    } = Self::expand_node_parts(prefix, value);

                    expanded.append_all(hquote! {
                        if req.reader.is_dangling_slash() #guards_expanded {
                            #handler_expanded
                        }
                    });
                }

                if !node.is_leaf() {
                    let recur = self.expand_nodes(full_path, node.children());
                    if suffix.is_empty() {
                        expanded.append_all(recur);
                    } else {
                        expanded.append_all(quote_reader_fallback! {
                            if req.reader.peek(#suffix_len) == #suffix {
                                req.reader.read(#suffix_len);
                                #recur
                            }
                        });
                    }
                }
            }
            _ => {
                let captures = Self::capture_param_parts(suffix);
                let conseq_expanded = match captures {
                    Some((prefix, param, suffix)) => {
                        self.expand_param_node(full_path, node, prefix, param, suffix)
                    }
                    None => hquote! {},
                };

                if !conseq_expanded.is_empty() {
                    expanded.append_all(conseq_expanded);
                } else if conseq_expanded.is_empty() && node.is_leaf() {
                    if let Some(value) = &node.value {
                        let ExpandedNodeParts {
                            guards_expanded,
                            handler_expanded,
                        } = Self::expand_node_parts(suffix, value);

                        expanded.append_all(quote_reader_fallback! {
                            if req.reader.peek(#suffix_len) == #suffix #guards_expanded {
                                req.reader.read(#suffix_len);
                                #handler_expanded
                            }
                        });
                    }
                } else {
                    let recur = self.expand_nodes(full_path, node.children());

                    if let Some(value) = &node.value {
                        let ExpandedNodeParts {
                            guards_expanded,
                            handler_expanded,
                        } = Self::expand_node_parts(suffix, value);

                        expanded.append_all(quote_reader_fallback! {
                            if req.reader.peek(#suffix_len) == #suffix #guards_expanded {
                                req.reader.read(#suffix_len);
                                #recur
                                #handler_expanded
                            }
                        });
                    } else {
                        expanded.append_all(recur);
                    }
                };
            }
        }

        // now we insert parsing of param
        expanded = quote_reader_fallback! {
            let param = req.reader.read_param();
            if let Ok(value) = param {
                req.params.push(#param.to_string(), value.to_string());
                #expanded
            }
        };

        // now we wrap everything with matching the literal before
        let prefix_len = prefix.len();
        if !prefix.is_empty() {
            expanded = quote_reader_fallback! {
                if req.reader.peek(#prefix_len) == #prefix {
                    req.reader.read(#prefix_len);
                    #expanded
                }
            }
        }

        expanded
    }

    /// Expand a node guards and handler.
    fn expand_node_parts(
        prefix: &str,
        TrieValue {
            guards: guard,
            handler,
            method,
            middleware,
            node_type,
        }: &TrieValue<'r>,
    ) -> ExpandedNodeParts {
        let guards_expanded = guard
            .iter()
            .fold(hquote! {}, |acc, guard| hquote! { #acc && #guard });

        let handler_expanded = match node_type {
            NodeType::Handler => Self::expand_handler(method, handler, middleware),
            NodeType::Subrouter => {
                let expanded = Self::expand_subrouter(handler, middleware);
                if prefix.ends_with('/') {
                    quote_reader_fallback! {
                        req.reader.read_back(1);

                        #expanded
                    }
                } else {
                    expanded
                }
            }
        };

        ExpandedNodeParts {
            guards_expanded,
            handler_expanded,
        }
    }

    /// Expand a handler.
    fn expand_handler(
        method: &'r Option<Method>,
        handler: &ItemHandler,
        middleware: &[&'r ItemUseMiddleware],
    ) -> TokenStream {
        let expanded = match handler {
            ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                let handler = match handler {
                    ItemHandler::Fn(handler) => hquote! { #handler },
                    ItemHandler::Macro(item_macro) => hquote! { (#item_macro) },
                    ItemHandler::SubRouter(_) => unreachable!(),
                };

                let middleware_expanded = Self::expand_middleware(
                    middleware,
                    hquote! {
                        ::submillisecond::IntoResponse::into_response(
                            ::submillisecond::Handler::handle(#handler, req)
                        )
                    },
                );

                hquote! {
                    if req.reader.is_dangling_slash() {
                        return #middleware_expanded;
                    }
                }
            }
            ItemHandler::SubRouter(_) => Self::expand_subrouter(handler, middleware),
        };

        match method {
            Some(method) => hquote! {
                let _ = ::http::Method::#method;
                if req.reader.is_empty(true) {
                    #expanded
                }
            },
            None => expanded,
        }
    }

    /// Expand a subrouter.
    fn expand_subrouter(
        handler: &ItemHandler,
        middleware: &[&'r ItemUseMiddleware],
    ) -> TokenStream {
        match handler {
            ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                let handler = match handler {
                    ItemHandler::Fn(handler) => hquote! { #handler },
                    ItemHandler::Macro(item_macro) => hquote! { (#item_macro) },
                    ItemHandler::SubRouter(_) => unreachable!(),
                };

                let middleware_expanded = Self::expand_middleware(
                    middleware,
                    hquote! {
                        ::submillisecond::IntoResponse::into_response(
                            ::submillisecond::Handler::handle(#handler, req)
                        )
                    },
                );

                hquote! {
                    return #middleware_expanded;
                }
            }
            ItemHandler::SubRouter(subrouter) => {
                let subrouter_expanded = subrouter.expand();

                let middleware_expanded = Self::expand_middleware(
                    middleware,
                    hquote! {
                        let subrouter = #subrouter_expanded;
                        ::submillisecond::IntoResponse::into_response(
                            ::submillisecond::Handler::handle(subrouter, req)
                        )
                    },
                );

                hquote! {
                    return #middleware_expanded;
                }
            }
        }
    }

    /// Expand middleware to inject into local static middleware vec.
    fn expand_middleware(middleware: &[&'r ItemUseMiddleware], inner: TokenStream) -> TokenStream {
        middleware
            .iter()
            .flat_map(|middleware| middleware.tree.items())
            .rev()
            .fold(hquote! {{ #inner }}, |acc, middleware| {
                hquote! {
                    ::submillisecond::Middleware::apply(&#middleware, req, move |req| {
                        #acc
                    })
                }
            })
    }

    /// Expand catch all handler, or if not present, return `RouteError::RouteNotMatch`.
    fn expand_catch_all(&self) -> TokenStream {
        match self.catch_all {
            Some(catch_all) => match &*catch_all.handler {
                ItemHandler::Fn(_) | ItemHandler::Macro(_) => {
                    let handler = match &*catch_all.handler {
                        ItemHandler::Fn(handler) => hquote! { #handler },
                        ItemHandler::Macro(item_macro) => hquote! { (#item_macro) },
                        ItemHandler::SubRouter(_) => unreachable!(),
                    };

                    hquote! {
                        return ::submillisecond::IntoResponse::into_response(
                            ::submillisecond::Handler::handle(#handler, req)
                        );
                    }
                }
                ItemHandler::SubRouter(subrouter) => {
                    let handler = subrouter.expand();

                    hquote! {
                        return ::submillisecond::IntoResponse::into_response(
                            ::submillisecond::Handler::handle(#handler, req)
                        );
                    }
                }
            },
            None => {
                hquote! { return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(req)); }
            }
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

    /// Recursively collect handlers and subrouters.
    fn collect_tries(
        &mut self,
        prefix: Option<String>,
        routes: &'r [ItemRoute],
        all_middleware: Vec<&'r ItemUseMiddleware>,
        all_guards: Vec<&'r ItemGuard>,
        catch_all: &'r Option<ItemCatchAll>,
    ) {
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

            let mut all_guards = all_guards.clone();
            if let Some(guard) = guard {
                all_guards.push(guard);
            }

            match handler {
                ItemHandler::SubRouter(Router::Tree(tree)) if catch_all.is_none() => {
                    self.collect_tries(
                        Some(new_path),
                        &tree.routes,
                        all_middleware,
                        all_guards,
                        &tree.catch_all,
                    );
                }
                _ => {
                    let value = TrieValue {
                        guards: all_guards,
                        handler,
                        method,
                        middleware: all_middleware,
                        node_type: if method.is_some() {
                            NodeType::Handler
                        } else {
                            NodeType::Subrouter
                        },
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

    fn capture_param_parts(s: &str) -> Option<(&str, &str, &str)> {
        RE.captures(s).map(|captures| {
            (
                captures.get(1).unwrap().as_str(),
                captures.get(2).unwrap().as_str(),
                captures.get(3).unwrap().as_str(),
            )
        })
    }
}
