mod item_route;
mod item_use_middleware;
pub mod method;

use std::str::FromStr;

use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::quote;
use submillisecond_core::router::tree::{Node, NodeType};
use radix_trie::{Trie, iter::Children, TrieCommon, SubTrie};
use syn::{
    parse::{Parse, ParseStream},
    LitStr, Token, Index,
};
use regex::Regex;
use rust_format::{Formatter, RustFmt};

use self::{
    item_route::{ItemHandler, ItemRoute},
    item_use_middleware::ItemUseMiddleware,
    method::Method,
};

pub struct MethodTries {
    get: Trie<String, (TokenStream, TokenStream)>,
    post: Trie<String, (TokenStream, TokenStream)>,
    put: Trie<String, (TokenStream, TokenStream)>,
    delete: Trie<String, (TokenStream, TokenStream)>,
    head: Trie<String, (TokenStream, TokenStream)>,
    options: Trie<String, (TokenStream, TokenStream)>,
    patch: Trie<String, (TokenStream, TokenStream)>,
}

impl MethodTries {
    pub fn new() -> MethodTries {
        MethodTries {
            get: Trie::new(),
            post: Trie::new(),
            put: Trie::new(),
            delete: Trie::new(),
            head: Trie::new(),
            options: Trie::new(),
            patch: Trie::new()
        }
    }

    pub fn insert(&mut self, method: Method, key: String, value: (TokenStream, TokenStream)) {
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

    pub fn expand(self) -> TokenStream {
        let expanded_method_arms = self.expand_method_arms();

        // TODO: maybe add some hooks to give devs ability to log requests that were sent but failed
        // to parse (also useful for us in case we need to debug)
        let wrapped = quote! {
            struct MyApp;

            impl ::submillisecond::core::WebApp for MyApp {
                #expanded_method_arms
            }
        };
        // println!("GOT EXPANDED {}", wrapped.to_string());
        // println!("GOT EXPANDED {}", RustFmt::default().format_tokens(wrapped.clone()).unwrap());
        TokenStream::from_str(&RustFmt::default().format_tokens(wrapped.clone()).unwrap()).unwrap()
    }

    fn expand_method_arms(&self) -> TokenStream {
        let pairs = [
            (quote! { handle_get_request }, Self::expand_method_trie(vec![], self.get.children())),
            (quote! { handle_post_request }, Self::expand_method_trie(vec![], self.post.children())),
            (quote! { handle_put_request }, Self::expand_method_trie(vec![], self.put.children())),
            (quote! { handle_delete_request }, Self::expand_method_trie(vec![], self.delete.children())),
            (quote! { handle_head_request }, Self::expand_method_trie(vec![], self.head.children())),
            (quote! { handle_options_request }, Self::expand_method_trie(vec![], self.options.children())),
            (quote! { handle_patch_request }, Self::expand_method_trie(vec![], self.patch.children()))];
        
            Self::expand_method_trie(vec![], self.post.children());

        // build expanded per-method match, only implement if at least one route for method is defined, otherwise
        // fall back to default impl
        let expanded = pairs.iter().filter(|(_, arms)| {!arms.is_empty()}).map(|(method, arms)| {
            quote! {
                fn #method(mut request: ::submillisecond::Request, params: &mut ::submillisecond_core::router::params::Params) -> Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                    let path = request.uri().path().to_string();
                    let mut reader = ::submillisecond::core::UriReader::new(path);
                    #( #arms )*
                    return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request));
                }
            }
        });

        quote! {
            #( #expanded )*
        }
    }

    fn expand_param_child(child: SubTrie<String, (TokenStream, TokenStream)>, captures: Vec<(String, String, String)>, full_path: Vec<u8>) -> TokenStream {
        let mut output = quote! {};

        // iterate in reverse because we need to nest if statements
        for (lit_prefix, param, lit_suffix) in captures.iter().rev() {
            // first we try to handle the suffix since if there's a static match after a param
            // we want to insert that static match as innermost part of our if statement
            println!("Get captured {} | {} | {}", lit_prefix.is_empty(), param, lit_suffix);
            let len = lit_suffix.len();
            if !lit_suffix.is_empty() {
                // handle case when the child is both a non-leaf and has a value
                if child.value().is_some() && !child.is_leaf() {
                    println!();
                    let (condition_ext, block) = child.value().unwrap();
                    let recur = Self::expand_method_trie(full_path.clone(), child.children());
                    output = if lit_suffix == "/" {
                        // skip the mandatory read because
                        quote! {
                            #output
                            if reader.is_dangling_slash() #condition_ext {
                                #block
                            }
                            if reader.read(1) != "/" {
                                return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request));
                            }
                            #( #recur )*
                        }
                    } else {
                        quote! {
                            #output
                            if reader.ends_with(#lit_suffix) #condition_ext {
                                reader.read(#len);
                                #block
                            }
                            if reader.peek(#len) == #lit_suffix #condition_ext {
                                reader.read(#len);
                                #( #recur )*
                            }
                        }
                    }
                } else if let Some((condition_ext, block)) = child.value() {
                    output = if lit_suffix == "/" {
                        // skip the mandatory read because we have a dangling slash
                        quote! {
                            #output
                            if reader.is_dangling_slash() #condition_ext {
                                #block
                            }
                        }
                    } else {
                        quote! {
                            #output
                            if reader.peek(#len) == #lit_suffix {
                                reader.read(#len);
                                #block
                            }
                        }
                    }
                } else if !child.is_leaf() {
                    // wrap output
                    let recur = Self::expand_method_trie(full_path.clone(), child.children());
                    output = if lit_suffix == "/" {
                        // skip the mandatory read because
                        quote! {
                            #output
                            #( #recur )*
                        }
                    } else {
                        quote! {
                            if reader.peek(#len) == #lit_suffix {
                                reader.read(#len);
                                #output
                                #( #recur )*
                            }
                        }
                    }
                }
            }
            // now we insert parsing of param
            output = quote! {
                let param = reader.read_param();
                if let Ok(p) = param {
                    params.push(#param .to_string(), p.to_string());
                    #output
                }
            };
            // now we wrap everything with matching the literal before
            let len = lit_prefix.len();
            if !lit_prefix.is_empty() {
                // wrap output
                output = quote! {
                    if reader.peek(#len) == #lit_prefix {
                        reader.read(#len);
                        #output
                    }
                }
            }
        }
        output
    
    }

    fn expand_node_with_value(
        path: String,
        id_str: String,
        source: TokenStream,
        (condition_ext, block): &(TokenStream, TokenStream)) -> TokenStream {
        let len = path.len();
        if path.ends_with("/") {
            let (path, len) = (path[0..(path.len() - 1)].to_string(), path.len() - 1);
            return quote! {
                if reader.peek(#len) == #path #condition_ext {
                    reader.read(#len);
                    if reader.is_dangling_slash() {
                        #block
                    }
                    // since path continues there has to be a separator
                    if reader.read(1) != "/" {
                        return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request));
                    }
                    #source
                }
            };
        }

        println!("INJECTING HANDLER {} | id {:?}", path, id_str);
        quote! {
            if reader.peek(#len) == #path #condition_ext {
                reader.read(#len);
                if reader.is_empty() {
                    #block
                }
                #source
            }
        }
            
        
    }

    fn expand_method_trie(full_path: Vec<u8>, children: Children<String, (TokenStream, TokenStream)>) -> Vec<TokenStream> {
        // children.for_each(|child| {
        // let is_only_child = children.collect::<SubTrie<String, String>>().len();
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?P<lit_prefix>[^:]*):(?P<param>[a-zA-Z_]+)(?P<lit_suffix>.*)").unwrap();
        }
        children.map(|child| {
            let prefix = child.prefix().as_bytes().to_owned();
            let path = String::from_utf8(prefix.clone()).unwrap();
            let id = [full_path.clone(), prefix.clone()].concat();
            let id_str = String::from_utf8(id.clone()).unwrap();
            let captures = RE.captures_iter(&path)
                                                    .map(|m| (m[1].to_string(), m[2].to_string(), m[3].to_string()))
                                                    .collect::<Vec<(String, String, String)>>();
                                                                
            // split longest common prefix at param and insert param matching 
            println!(
                "=========================\nPROCESSING CHILD {:?} -> {:?} | is empty {} | {:?}\n=====================",
                String::from_utf8(id.clone()),
                path,
                captures.is_empty(),
                captures,
            );
            // split match by param
            if !captures.is_empty() {
                return Self::expand_param_child(child, captures, id);
            }
            let len = path.len();
            println!("Doing regular non-param matching {:?} | {} | is_leaf: {} | has_value {}", id_str, path, child.is_leaf(), child.value().is_some());

            // recursive expand if not leaf
            if !child.is_leaf() {
                let recur = Self::expand_method_trie(id.clone(), child.children());
                if let Some(v) = child.value() {
                    return Self::expand_node_with_value(path, id_str, quote! {
                        #( #recur )*
                    }, v);
                }
                return quote! {
                    if reader.peek(#len) == #path {
                        reader.read(#len);
                        #( #recur )*
                    }
                };
            } else if let Some(v) = child.value() {
                return Self::expand_node_with_value(path, id_str, quote! {}, v);
            }
            quote! {}
            // Self::expand_method_trie(format!("    {}", indent), id, child.children());
            // println!("{}}}", indent);
        }).collect()
    }
}

#[derive(Debug)]
pub struct RouterTree {
    middleware: Vec<ItemUseMiddleware>,
    routes: Vec<ItemRoute>,
}

impl RouterTree {
    pub fn expand(&self, trie: &mut MethodTries, prefix: Option<&LitStr>) -> TokenStream {
        // let middleware_expanded = self.middleware().into_iter().map(|middleware| {});

        println!("EXPANDING ROUTER TREE {:?}", prefix);
        let mut router_node = Node::default();
        for route in &self.routes {
            let mut path = route.path.value();
            if route.method.is_none() {
                path.push_str("/*__slug");
            } else {
                // router.insert(format!("{}||{}", route.method.unwrap(), new_path.clone()), new_path.clone());
                // println!("INSERTING TRIE {:?}", path);
                // trie.insert(path.clone(), path.clone());
            }
            // if let ItemHandler::SubRouter(sub_router) = route.handler {
            //     sub_router.routes
            //     router_node.children.append(sub_router.expand_router_nodes(prefix));
            // }
            let lit = LitStr::new(&path, route.path.span());
            if let Err(err) = router_node.insert(path, (route.method, lit)) {
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
                let new_path = if let Some(p) = prefix {
                    let mut s = p.value();
                    s.push_str(&path.value());
                    LitStr::new(&s, path.span())
                } else {
                    path.clone()
                };
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
                        |method| quote! { method.as_ref().map(|method| method == #method).unwrap_or(false) },
                    )
                    .unwrap_or_else(|| quote! { true });

                let guards_expanded = guard
                    .as_ref()
                    .map(|guard| &*guard.guard)
                    .map(|guard| quote! { && { #guard } });

                let (middleware_before, middleware_after) = if let Some(m) = middleware {
                    let items = m.tree.items();
                    let invoke_middleware = items
                        .iter()
                        .map(|item| {
                            quote! {
                                <#item as ::submillisecond::Middleware>::before(&mut request)
                            }
                        });

                    let before_calls = quote! {
                        let middleware_calls = ( #( #invoke_middleware, )* );
                    };
                    
                    let after_calls = (0..items.len())
                        .map(|idx| {
                            let idx = Index::from(idx);
                            quote! {
                                middleware_calls.#idx.after(&mut res);
                            }
                        });

                    (before_calls, quote! {#( #after_calls )*})
                } else {
                    (quote! {}, quote! {})
                };

                match handler {
                    ItemHandler::Fn(handler_ident) => {
                        if let Some(method) = method {
                            trie.insert(*method, new_path.value(), (quote! {#guards_expanded}, quote! {
                                    Self::merge_extensions(&mut request, params);
                                    #middleware_before
    
                                    let mut res = ::submillisecond::response::IntoResponse::into_response(
                                        ::submillisecond::handler::Handler::handle(
                                            #handler_ident
                                                as ::submillisecond::handler::FnPtr<
                                                    _,
                                                    _,
                                                    { ::submillisecond::handler::arity(&#handler_ident) },
                                                >,
                                            request,
                                        ),
                                    );
    
                                    #middleware_after
    
                                    return ::std::result::Result::Ok(res);
                            }));
                        }
                        quote! {
                            #new_path if #method_expanded #guards_expanded => {
                                #middleware_before

                                let mut res = ::submillisecond::response::IntoResponse::into_response(
                                    ::submillisecond::handler::Handler::handle(
                                        #handler_ident
                                            as ::submillisecond::handler::FnPtr<
                                                _,
                                                _,
                                                { ::submillisecond::handler::arity(&#handler_ident) },
                                            >,
                                        request,
                                    ),
                                );

                                #middleware_after

                                return ::std::result::Result::Ok(res);
                            }
                        }
                    },
                    ItemHandler::SubRouter(sub_router) => {
                        let sub_router_expanded = sub_router.expand(trie, Some(&new_path));

                        sub_router_expanded
                        // quote! {
                        //     #path if #method_expanded #guards_expanded => {
                        //         const SUB_ROUTER: ::submillisecond::handler::HandlerFn = #sub_router_expanded;
                        //         return SUB_ROUTER(req);
                        //     }
                        // }
                    },
                }
            },
        );

        // prefix is only Some() if it's a subrouter
        if let Some(_) = prefix {
            return quote! {
                #( #arms_expanded )*
            }
        }

        quote! {
            (|mut request: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                const ROUTER: ::submillisecond_core::router::Router<'static, (::std::option::Option<::http::Method>, &'static str)> = ::submillisecond_core::router::Router::from_node(
                    #node_expanded,
                );

                let path = &request
                    .extensions()
                    .get::<::submillisecond::router::Route>()
                    .unwrap()
                    .0;
                let route_match = ROUTER.at(path);
                println!("Matching path {:?} | {:?}", path, route_match);
                match route_match {
                    ::std::result::Result::Ok(::submillisecond_core::router::Match {
                        params,
                        values,
                    }) => {
                        if !params.is_empty() {
                            if let Some(slug) = params.get("__slug") {
                                let mut path = ::std::string::String::with_capacity(slug.len() + 1);
                                path.push('/');
                                path.push_str(slug);

                                let route = request
                                    .extensions_mut()
                                    .get_mut::<::submillisecond::router::Route>()
                                    .unwrap();
                                *route = ::submillisecond::router::Route(path);
                            }

                            match request
                                .extensions_mut()
                                .get_mut::<::submillisecond_core::router::params::Params>()
                            {
                                ::std::option::Option::Some(params_ext) => params_ext.merge(params),
                                ::std::option::Option::None => {
                                    request.extensions_mut().insert(params);
                                }
                            }
                        }

                        for (method, route) in values {
                            match *route {
                                #( #arms_expanded, )*
                                _ => {},
                            }
                        }

                        ::std::result::Result::Err(
                            ::submillisecond::router::RouteError::RouteNotMatch(request),
                        )
                    }
                    ::std::result::Result::Err(_) => {
                        ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request))
                    }
                }
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
    }: &Node<(Option<Method>, LitStr)>,
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
                Method::Get(get) => quote! { ::std::option::Option::Some(::http::Method::#get) },
                Method::Post(post) => quote! { ::std::option::Option::Some(::http::Method::#post) },
                Method::Put(put) => quote! { ::std::option::Option::Some(::http::Method::#put) },
                Method::Delete(delete) => {
                    quote! { ::std::option::Option::Some(::http::Method::#delete) }
                }
                Method::Head(head) => quote! { ::std::option::Option::Some(::http::Method::#head) },
                Method::Options(options) => {
                    quote! { ::std::option::Option::Some(::http::Method::#options) }
                }
                Method::Patch(patch) => {
                    quote! { ::std::option::Option::Some(::http::Method::#patch) }
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
