mod item_route;
mod item_use_middleware;
pub mod method;

use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use radix_trie::{Trie, iter::Children, TrieCommon, SubTrie};
use syn::{
    parse::{Parse, ParseStream},
    Expr, Index, LitStr, Token,
};
use regex::Regex;

use crate::router::Router;

use self::{
    item_route::{ItemHandler, ItemRoute},
    item_use_middleware::ItemUseMiddleware,
    method::Method,
};

pub struct MethodTries {
    // tries to collect
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
            patch: Trie::new(),
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

    pub fn expand(mut self, router: RouterTree) -> TokenStream {
        self.collect_route_data(None, &router.routes);
        
        let middleware = router.middleware();
        let (middleware_before, middleware_after) = Self::get_middleware_calls(&middleware, false);
        let expanded_method_arms = self.expand_method_arms();

        // TODO: maybe add some hooks to give devs ability to log requests that were sent but failed
        // to parse (also useful for us in case we need to debug)
        let wrapped = quote! {
            |mut __req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                let mut params = ::submillisecond::params::Params::new();
                #middleware_before

                let mut __resp = match *__req.method() {
                    #expanded_method_arms
        
                    _ => ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
                };

                if let Ok(ref mut __resp) = &mut __resp {
                    #middleware_after
                }

                __resp
            }
        };
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
        
            Self::expand_method_trie(vec![], self.post.children());

        // build expanded per-method match, only implement if at least one route for method is defined, otherwise
        // fall back to default impl
        let expanded = pairs.iter().map(|(method, arms)| {
            if arms.is_empty() {
                return quote! {
                    #method => Err(::submillisecond::router::RouteError::RouteNotMatch(__req)),
                };
            }
            quote! {
                #method => {
                    let path = __req.uri().path().to_string();
                    let mut reader = ::submillisecond::core::UriReader::new(path);
                    #( #arms )*
                    return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req));
                }
            }
        });

        quote! {
            #( #expanded )*
        }
    }

    fn collect_route_data(&mut self, prefix: Option<&LitStr>, routes: &[ItemRoute]) {
        for ItemRoute {
                method,
                path,
                guard,
                middleware,
                handler,
                ..
            } in routes.iter() {
                let new_path = if let Some(p) = prefix {
                    let mut s = p.value();
                    s.push_str(&path.value());
                    LitStr::new(&s, path.span())
                } else {
                    path.clone()
                };

                let guards_expanded = guard
                    .as_ref()
                    .map(|guard| &*guard.guard)
                    .map(expand_guard_struct)
                    .map(|guard| quote! { && { #guard } });

                let mid = if let Some(m) = middleware {
                    m.tree.items()
                } else {
                    vec![]
                };
                let (middleware_before, middleware_after) = Self::get_middleware_calls(&mid, true);

                match handler {
                    ItemHandler::Fn(handler_ident) => {
                        if let Some(method) = method {
                            self.insert(*method, new_path.value(), (quote! {#guards_expanded}, quote! {
                                    ::submillisecond::Application::merge_extensions(&mut __req, &mut params);
                                    #middleware_before
    
                                    let mut __resp = ::submillisecond::response::IntoResponse::into_response(
                                        ::submillisecond::handler::Handler::handle(
                                            #handler_ident
                                                as ::submillisecond::handler::FnPtr<
                                                    _,
                                                    _,
                                                    { ::submillisecond::handler::arity(&#handler_ident) },
                                                >,
                                            __req,
                                        ),
                                    );
    
                                    #middleware_after
    
                                    return ::std::result::Result::Ok(__resp);
                            }));
                        }
                    },
                    ItemHandler::SubRouter(Router::Tree(tree)) => {
                        self.collect_route_data(Some(&new_path), &tree.routes);
                    },
                    other => println!("GOT SOMETHING ELSE {:?}", other)
                }
            }
        }

    fn get_middleware_calls(items: &[TokenStream], use_ref: bool) -> (TokenStream, TokenStream) {
        if !items.is_empty() {
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

            let response_ref = if use_ref {
                quote! {&mut __resp}
            } else {
                quote! {__resp}
            };
            let after_calls = (0..items.len())
                .map(|idx| {
                    let idx = Index::from(idx);
                    quote! {
                        ::submillisecond::Middleware::after(__middleware_calls.#idx, #response_ref);
                    }
                });

            (before_calls, quote! {#( #after_calls )*})
        } else {
            (quote! {}, quote! {})
        }
    }

    fn expand_param_child(child: SubTrie<String, (TokenStream, TokenStream)>, captures: Vec<(String, String, String)>, full_path: Vec<u8>) -> TokenStream {
        let mut output = quote! {};

        // iterate in reverse because we need to nest if statements
        for (lit_prefix, param, lit_suffix) in captures.iter().rev() {
            // first we try to handle the suffix since if there's a static match after a param
            // we want to insert that static match as innermost part of our if statement
            let len = lit_suffix.len();
            if !lit_suffix.is_empty() {
                // handle case when the child is both a non-leaf and has a value
                if child.value().is_some() && !child.is_leaf() {
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
                                return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req));
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
        source: TokenStream,
        (condition_ext, block): &(TokenStream, TokenStream)) -> TokenStream {
        let len = path.len();
        if path.ends_with('/') {
            let (path, len) = (path[0..(path.len() - 1)].to_string(), path.len() - 1);
            return quote! {
                if reader.peek(#len) == #path #condition_ext {
                    reader.read(#len);
                    if reader.is_dangling_slash() {
                        #block
                    }
                    // since path continues there has to be a separator
                    if reader.read(1) != "/" {
                        return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req));
                    }
                    #source
                }
            };
        }

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

    // fn unpack_nibbles(src: Nibble) -> Vec<u8> {
    //     return src;
    //     // if src.len() % 2 != 0 || src.is_empty() {
    //     //     return src;
    //     // }
    //     let mut out = Vec::with_capacity(src.len() / 2);
    //     println!("Unpacking nibbles {:?}", src);
    //     for idx in (0..src.len()).step_by(2) {
    //         println!("Doing index {:?} | hi {} | lo {}", idx, src[idx], src[idx+1]);
    //         out.push(src);
    //     }
    //     out
    // }

    fn expand_method_trie(full_path: Vec<u8>, children: Children<String, (TokenStream, TokenStream)>) -> Vec<TokenStream> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?P<lit_prefix>[^:]*):(?P<param>[a-zA-Z_]+)(?P<lit_suffix>.*)").unwrap();
        }
        children.map(|child| {
            // let prefix = child.prefix();
            // for c in (0..child.prefix().len()).step_by(2) {
            //     prefix.get(idx)
            // }
            println!("GOT CHILD PATH {:?} | nibble {:?}", child.prefix(), child.prefix().as_bytes());
            let prefix = child.prefix();
            let path = String::from_utf8(child.prefix().clone().into_bytes()).unwrap();
            println!("PARSED PATH {:?}", path);
            let id = [full_path.clone(), path.as_bytes().to_vec()].concat();
            let captures = RE.captures_iter(&path)
                                                    .map(|m| (m[1].to_string(), m[2].to_string(), m[3].to_string()))
                                                    .collect::<Vec<(String, String, String)>>();
                                                                
            // split longest common prefix at param and insert param matching 
            if !captures.is_empty() {
                return Self::expand_param_child(child, captures, id);
            }
            let len = path.len();

            // recursive expand if not leaf
            if !child.is_leaf() {
                let recur = Self::expand_method_trie(id, child.children());
                if let Some(v) = child.value() {
                    return Self::expand_node_with_value(path, quote! {
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
                return Self::expand_node_with_value(path, quote! {}, v);
            }
            quote! {}
        }).collect()
    }
}

#[derive(Debug)]
pub struct RouterTree {
    pub middleware: Vec<ItemUseMiddleware>,
    pub routes: Vec<ItemRoute>,
}

impl RouterTree {
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
