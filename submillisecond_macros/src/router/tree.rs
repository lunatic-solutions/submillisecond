mod item_route;
mod item_use_middleware;
pub mod method;

use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use submillisecond_core::router::tree::{Node, NodeType};
use radix_trie::{Trie, iter::Children, TrieCommon};
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

        let wrapped = quote! {
            fn match_request(reader: String) {
                let path = &req
                    .extensions()
                    .get::<::submillisecond::router::Route>()
                    .unwrap()
                    .0;
                let mut request = match core::parse_request(stream.clone()) {
                    Ok(request) => request,
                    Err(err) => {
                        if let Err(err) = core::write_response(stream, err.into_response()) {
                            eprintln!("[http reader] Failed to send response {:?}", err);
                        }
                        return;
                    }
                };

                let path = request.uri().path().to_string();
                let extensions = request.extensions_mut();
                extensions.insert(Route(path));
                let http_version = request.version();
                let reader = core::UriReader::new(path);

                let mut response = match request.method() {
                    #expanded_method_arms
                }.unwrap_or_else(|err| err.into_response());
                let content_length = response.body().len();
                *response.version_mut() = http_version;
                response
                    .headers_mut()
                    .append(header::CONTENT_LENGTH, HeaderValue::from(content_length));

                if let Err(err) = core::write_response(stream, response) {
                    eprintln!("[http reader] Failed to send response {:?}", err);
                }
            }
        };
        println!("GOT EXPANDED {}", RustFmt::default().format_tokens(wrapped.clone()).unwrap());
        wrapped
    }

    fn expand_method_arms(&self) -> TokenStream {
        let pairs = [
            (quote! { ::http::Method::GET }, Self::expand_method_trie(vec![], self.get.children())),
            (quote! { ::http::Method::POST }, Self::expand_method_trie(vec![], self.post.children())),
            (quote! { ::http::Method::PUT }, Self::expand_method_trie(vec![], self.put.children())),
            (quote! { ::http::Method::DELETE }, Self::expand_method_trie(vec![], self.delete.children())),
            (quote! { ::http::Method::HEAD }, Self::expand_method_trie(vec![], self.head.children())),
            (quote! { ::http::Method::OPTIONS }, Self::expand_method_trie(vec![], self.options.children())),
            (quote! { ::http::Method::PATCH }, Self::expand_method_trie(vec![], self.patch.children()))];

        // build expanded per-method match
        let expanded = pairs.iter().filter(|(_, arms)| {!arms.is_empty()}).map(|(method, arms)| {
            quote! {
                #method => {
                    #( #arms )*
                }
            }
        });

        quote! {
            #( #expanded )*,
            _ => RouteError::RouteNotMatch(request)
        }
    }

    fn expand_method_trie(full_path: Vec<u8>, children: Children<String, (TokenStream, TokenStream)>) -> Vec<TokenStream> {
        // children.for_each(|child| {
        // let is_only_child = children.collect::<SubTrie<String, String>>().len();
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?P<lit_prefix>[^:]*):(?P<param>[a-zA-Z_]+)(?P<lit_suffix>/|$)").unwrap();
        }
        children.map(|child| {
            let prefix = child.prefix().as_bytes().to_owned();
            let prefix_str = String::from_utf8(prefix.clone()).unwrap();
            let captures = RE.captures_iter(&prefix_str)
                                                    .map(|m| (m[1].to_string(), m[2].to_string(), m[3].to_string()))
                                                    .collect::<Vec<(String, String, String)>>();
            // split longest common prefix at param and insert param matching 
            println!(
                "GOT CHILD PARAMS {:?} {:?}",
                prefix_str,
                RE.captures(&prefix_str)
            );
            // split match by param
            if !captures.is_empty() {
                let mut output = quote! {};
                // iterate in reverse because we need to nest if statements
                for (lit_prefix, param, lit_suffix) in captures.iter().rev() {
                    // first we try to handle the suffix since if there's a static match after a param
                    // we want to insert that static match as innermost part of our if statement
                    let len = lit_suffix.len();
                    if !lit_suffix.is_empty() {
                        if child.is_leaf() {
                            let (condition_ext, block) = child.value().unwrap();
                            // handle dangling slash
                            let handle_dangling_slash = if lit_suffix == "/" {
                                quote! {|| peeked == ""}
                            } else { quote!{} };
                            output = quote! {
                                let peeked = reader.peek(#len);
                                if peeked == #lit_suffix #handle_dangling_slash #condition_ext {
                                    reader.read(#len);
                                    #block
                                }
                            }
                        } else {
                            // wrap output
                            output = quote! {
                                if reader.peek(#len) == #lit_suffix {
                                    reader.read(#len);
                                    #output
                                }
                            }
                        }
                    }
                    // now we insert parsing of param
                    output = quote! {
                        let param = reader.read_param();
                        if let Ok(_) = param {
                            params.insert(#param, param);
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
                return output;
            }
            // let captures = RE.cap
            let id = [full_path.clone(), prefix.clone()].concat();
            let len = prefix.len();
            let path = String::from_utf8(prefix).unwrap();
            
            if child.is_leaf() {
                let (condition_ext, block) = child.value().unwrap();
                let source = quote! {
                    if reader.peek(#len) == #path #condition_ext {
                        reader.read(#len);
                        #block
                    }
                };
                return source;
            } else {
                let recur = Self::expand_method_trie(id, child.children());
                return quote! {
                    if reader.peek(#len) == #path {
                        reader.read(#len);
                        #( #recur )*
                    }
                };
            }
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
                                <#item as ::submillisecond::Middleware>::before(&mut req)
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
                                    #middleware_before
    
                                    let mut res = ::submillisecond::response::IntoResponse::into_response(
                                        ::submillisecond::handler::Handler::handle(
                                            #handler_ident
                                                as ::submillisecond::handler::FnPtr<
                                                    _,
                                                    _,
                                                    { ::submillisecond::handler::arity(&#handler_ident) },
                                                >,
                                            req,
                                        ),
                                    );
    
                                    #middleware_after
    
                                    ::std::result::Result::Ok(res)
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
                                        req,
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
            (|mut req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                const ROUTER: ::submillisecond_core::router::Router<'static, (::std::option::Option<::http::Method>, &'static str)> = ::submillisecond_core::router::Router::from_node(
                    #node_expanded,
                );

                let path = &req
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

                                let route = req
                                    .extensions_mut()
                                    .get_mut::<::submillisecond::router::Route>()
                                    .unwrap();
                                *route = ::submillisecond::router::Route(path);
                            }

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

                        for (method, route) in values {
                            match *route {
                                #( #arms_expanded, )*
                                _ => {},
                            }
                        }

                        ::std::result::Result::Err(
                            ::submillisecond::router::RouteError::RouteNotMatch(req),
                        )
                    }
                    ::std::result::Result::Err(_) => {
                        ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(req))
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
    println!("Expanding router nodes {:?} | {:?}", String::from_utf8(prefix.to_vec()), value);
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
