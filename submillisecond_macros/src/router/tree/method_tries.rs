use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use regex::Regex;
use syn::{Expr, LitStr};

use crate::router::Router;

use super::{
    item_route::{ItemHandler, ItemRoute},
    method::Method,
    trie::{Children, Node, Trie},
    RouterTree,
};

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?P<lit_prefix>[^:]*):(?P<param>[a-zA-Z_]+)(?P<lit_suffix>.*)").unwrap();
}

#[derive(Debug, Default)]
pub struct MethodTries {
    // trie to collect subrouters
    subrouters: Trie<MethodValue>,
    // tries to collect
    get: Trie<MethodValue>,
    post: Trie<MethodValue>,
    put: Trie<MethodValue>,
    delete: Trie<MethodValue>,
    head: Trie<MethodValue>,
    options: Trie<MethodValue>,
    patch: Trie<MethodValue>,
}

#[derive(Clone, Debug)]
struct MethodValue {
    method: Option<Method>,
    guards: Option<TokenStream>,
    body: TokenStream,
}

impl MethodTries {
    pub fn new(router: &RouterTree) -> Self {
        let mut method_tries = MethodTries::default();
        method_tries.collect_route_data(None, &router.routes, None, router.middleware());
        method_tries
    }

    pub fn expand(mut self) -> TokenStream {
        let expanded_method_arms = self.expand_method_arms();
        let (subrouter_expanded, maybe_reset) = self.expand_subrouter();

        quote! {
            #subrouter_expanded

            // need to reset reader after failing to match subrouters
            #maybe_reset
            match *__req.method() {
                #expanded_method_arms

                _ => ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req)),
            }
        }
    }

    fn insert(&mut self, method: Method, key: String, value: MethodValue) {
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

    fn insert_subrouter(&mut self, key: String, value: MethodValue) {
        self.subrouters.insert(key, value);
    }

    fn expand_subrouter(&mut self) -> (TokenStream, TokenStream) {
        let expanded = Self::expand_method_trie(vec![], self.subrouters.children());
        (
            quote! {
                #( #expanded )*
            },
            if expanded.is_empty() {
                quote! {}
            } else {
                quote! {__reader.reset();}
            },
        )
    }

    fn expand_method_arms(&mut self) -> TokenStream {
        let pairs = [
            (quote! { ::http::Method::GET }, self.get.children()),
            (quote! { ::http::Method::POST }, self.post.children()),
            (quote! { ::http::Method::PUT }, self.put.children()),
            (quote! { ::http::Method::DELETE }, self.delete.children()),
            (quote! { ::http::Method::HEAD }, self.head.children()),
            (quote! { ::http::Method::OPTIONS }, self.options.children()),
            (quote! { ::http::Method::PATCH }, self.patch.children()),
        ];

        // build expanded per-method match, only implement if at least one route for method is defined, otherwise
        // fall back to default impl
        let expanded = pairs.into_iter().filter_map(|(method, children)| {
            let arms = Self::expand_method_trie(vec![], children.clone());
            if arms.is_empty() {
                return None;
            }

            fn flatten_methods(methods: &mut Vec<Method>, children: Children<MethodValue>) {
                for node in children {
                    if let Some(value) = &node.value {
                        if let Some(method) = &value.method {
                            methods.push(*method);
                        }
                    }

                    if !node.is_leaf() {
                        flatten_methods(methods, node.children());
                    }
                }
            }

            let mut methods = Vec::new();
            flatten_methods(&mut methods, children);

            Some(quote! {
                #method => {
                    // Forward spans for method prefix syntax highlighting.
                    // Without this, the GET/POST keywords wouldn't be highlighted correctly in the IDE.
                    #( let _ = ::http::Method::#methods; )*
                    #( #arms )*
                    return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req));
                }
            })
        });

        quote! {
            #( #expanded )*
        }
    }

    fn collect_route_data(
        &mut self,
        prefix: Option<&LitStr>,
        routes: &[ItemRoute],
        ancestor_guards: Option<TokenStream>,
        parent_middlewares: Vec<TokenStream>,
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
            let new_path = if let Some(p) = prefix {
                let mut s = p.value();
                s.push_str(&path.value());
                LitStr::new(&s, path.span())
            } else {
                path.clone()
            };

            let mut guards_expanded = guard
                .as_ref()
                .map(|guard| &*guard.guard)
                .map(expand_guard_struct)
                .map(|guard| quote! { && { #guard } });

            if let Some(ref ancestor) = ancestor_guards {
                if let Some(guards) = guards_expanded {
                    guards_expanded = Some(quote! {#ancestor #guards});
                } else {
                    guards_expanded = Some(ancestor.clone());
                }
            }

            let mid = if let Some(m) = middleware {
                m.tree.items()
            } else {
                vec![]
            };
            let mut full_middlewares = parent_middlewares.clone();
            full_middlewares.extend_from_slice(&mid);
            let full_middlewares_expanded = Self::get_middleware_calls(&full_middlewares, true);

            match handler {
                ItemHandler::Fn(handler_ident) => {
                    if let Some(method) = method {
                        let value = MethodValue {
                            method: Some(*method),
                            guards: guards_expanded,
                            body: quote! {
                                if __reader.is_dangling_slash() {
                                    ::submillisecond::Application::merge_extensions(&mut __req, &mut __params);

                                    #full_middlewares_expanded

                                    return ::std::result::Result::Ok(
                                        ::submillisecond::response::IntoResponse::into_response(
                                            ::submillisecond::handler::Handler::handle(
                                                #handler_ident
                                                    as ::submillisecond::handler::FnPtr<
                                                        _,
                                                        _,
                                                        { ::submillisecond::handler::arity(&#handler_ident) },
                                                    >,
                                                __req
                                            ),
                                        )
                                    );
                                }
                            },
                        };
                        self.insert(*method, new_path.value(), value);
                    } else {
                        let value = MethodValue {
                            method: None,
                            guards: guards_expanded,
                            body: quote! {
                                ::submillisecond::Application::merge_extensions(&mut __req, &mut __params);

                                #full_middlewares_expanded
                                return #handler_ident(__req, __params, __reader);
                            },
                        };
                        self.insert_subrouter(new_path.value(), value);
                    }
                }
                ItemHandler::Macro(macro_expanded) => {
                    let value = MethodValue {
                        method: None,
                        guards: guards_expanded,
                        body: quote! {
                            ::submillisecond::Application::merge_extensions(&mut __req, &mut __params);

                            #full_middlewares_expanded
                            #macro_expanded
                        },
                    };
                    self.insert_subrouter(new_path.value(), value);
                }
                ItemHandler::SubRouter(Router::Tree(tree)) => {
                    self.collect_route_data(
                        Some(&new_path),
                        &tree.routes,
                        guards_expanded,
                        full_middlewares.clone(),
                    );
                }
                ItemHandler::SubRouter(Router::List(list)) => {
                    let value = MethodValue {
                        method: None,
                        guards: guards_expanded,
                        body: list.expand_inner(&full_middlewares),
                    };
                    self.insert_subrouter(new_path.value(), value);
                }
            }
        }
    }

    fn get_middleware_calls(items: &[TokenStream], _use_ref: bool) -> TokenStream {
        let before_calls = items
            .iter()
            .map(|item| {
                quote! {
                     ::submillisecond::request_context::inject_middleware(Box::new(<#item as Default>::default()));
                }
            });
        quote! { #( #before_calls )* }
    }

    fn expand_param_child(
        child: Node<MethodValue>,
        (lit_prefix, param, lit_suffix): (String, String, String),
        full_path: Vec<u8>,
    ) -> TokenStream {
        let mut output = quote! {};

        // iterate in reverse because we need to nest if statements
        // for (lit_prefix, param, lit_suffix) in captures.iter().rev() {
        // first we try to handle the suffix since if there's a static match after a param
        // we want to insert that static match as innermost part of our if statement
        let len = lit_suffix.len();
        match lit_suffix.as_str() {
            "" => {
                if let Some(MethodValue {
                    method: _,
                    guards,
                    body,
                }) = child.value.as_ref()
                {
                    output = quote! {
                        if __reader.is_dangling_slash() #guards {
                            #body
                        }
                    };
                }
                if !child.is_leaf() {
                    let recur = Self::expand_method_trie(full_path, child.children());
                    output = quote! {
                        #output
                        #( #recur )*
                    };
                }
            }
            "/" => {
                if let Some(MethodValue {
                    method: _,
                    guards,
                    body,
                }) = child.value.as_ref()
                {
                    output = quote! {
                        if __reader.is_dangling_slash() #guards {
                            #body
                        }
                    };
                }
                // if there's further matching going on we need to strict match the slash
                if !child.is_leaf() {
                    let recur = Self::expand_method_trie(full_path, child.children());
                    output = quote! {
                        #output
                        if __reader.peek(1) == "/" {
                            __reader.read(1);
                            #( #recur )*
                        }
                    };
                }
            }
            _ => {
                let consequent_params = RE
                    .captures(&lit_suffix)
                    .map(|m| (m[1].to_string(), m[2].to_string(), m[3].to_string()));
                let conseq_expanded = if let Some(consequent_params) = consequent_params {
                    Self::expand_param_child(child.clone(), consequent_params, full_path.clone())
                } else {
                    quote! {}
                };
                let body = if !conseq_expanded.is_empty() {
                    conseq_expanded
                } else if conseq_expanded.is_empty() && child.is_leaf() {
                    if let Some(MethodValue {
                        method: _,
                        guards,
                        body,
                    }) = child.value.as_ref()
                    {
                        quote! {
                            if __reader.peek(#len) == #lit_suffix #guards {
                                __reader.read(#len);
                                #body
                            }
                        }
                    } else {
                        quote! {}
                    }
                } else {
                    let recur = Self::expand_method_trie(full_path, child.children());
                    if let Some(MethodValue {
                        method: _,
                        guards,
                        body,
                    }) = child.value.as_ref()
                    {
                        quote! {
                            if __reader.peek(#len) == #lit_suffix #guards {
                                __reader.read(#len);
                                #body
                                #( #recur )*
                            }
                        }
                    } else {
                        quote! { #( #recur )* }
                    }
                };
                output = quote! {
                    #output
                    #body
                };
            }
        }

        // now we insert parsing of param
        output = quote! {
            let param = __reader.read_param();
            if let Ok(p) = param {
                __params.push(#param .to_string(), p.to_string());
                #output
            }
        };
        // now we wrap everything with matching the literal before
        let len = lit_prefix.len();
        if !lit_prefix.is_empty() {
            // wrap output
            output = quote! {
                if __reader.peek(#len) == #lit_prefix {
                    __reader.read(#len);
                    #output
                }
            }
        }
        // }
        output
    }

    fn expand_node_with_value(
        path: String,
        source: TokenStream,
        MethodValue {
            method: _,
            guards,
            body,
        }: &MethodValue,
    ) -> TokenStream {
        let len = path.len();
        if path.len() > 1 && path.ends_with('/') {
            let (path, len) = (path[0..(path.len() - 1)].to_string(), path.len() - 1);
            return quote! {
                if __reader.peek(#len) == #path #guards {
                    __reader.read(#len);
                    #body

                        // since path continues there has to be a separator
                    if !__reader.ensure_next_slash() {
                        return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(__req));
                    }
                    #source
                }
            };
        }

        quote! {
            if __reader.peek(#len) == #path #guards {
                __reader.read(#len);
                #body

                #source
            }
        }
    }

    fn expand_method_trie(full_path: Vec<u8>, children: Children<MethodValue>) -> Vec<TokenStream> {
        children
            .map(|child| {
                let path = String::from_utf8(child.prefix.clone()).unwrap();
                let id = [full_path.clone(), path.as_bytes().to_vec()].concat();
                let captures = RE
                    .captures(&path)
                    .map(|m| (m[1].to_string(), m[2].to_string(), m[3].to_string()));

                // split longest common prefix at param and insert param matching
                if let Some(captures) = captures {
                    return Self::expand_param_child(child, captures, id);
                }
                let len = path.len();

                // recursive expand if not leaf
                if !child.is_leaf() {
                    let recur = Self::expand_method_trie(id, child.children());
                    if let Some(v) = child.value {
                        return Self::expand_node_with_value(
                            path,
                            quote! {
                                #( #recur )*
                            },
                            &v,
                        );
                    }
                    return quote! {
                        if __reader.peek(#len) == #path {
                            __reader.read(#len);
                            #( #recur )*
                        }
                    };
                } else if let Some(v) = child.value {
                    return Self::expand_node_with_value(path, quote! {}, &v);
                }
                quote! {}
            })
            .collect()
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
