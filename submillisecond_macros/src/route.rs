use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    FnArg, Ident, ItemFn, LitStr, Pat, PatType, ReturnType, Type,
};

const REQUEST_TYPES: [&str; 3] = [
    "::submillisecond::Request",
    "submillisecond::Request",
    "Request",
];

pub struct Route {
    attrs: RouteAttrs,
    item_fn: ItemFn,
    req_pat: Option<(Pat, Type)>,
    extractors: Vec<(Pat, Type)>,
    return_ty: Option<Type>,
}

impl Route {
    pub fn parse_with_attributes(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        let attrs = syn::parse(attr)?;
        let mut item_fn: ItemFn = syn::parse(item)?;

        // Get request param and overwrite it with submillisecond::Request
        let mut req_pat = None;

        let extractors = item_fn
            .sig
            .inputs
            .iter()
            .filter_map(|input| match input {
                FnArg::Receiver(_) => Some(Err(syn::Error::new(
                    input.span(),
                    "routes cannot take self",
                ))),
                FnArg::Typed(PatType { pat, ty, .. }) => {
                    let ty_string = ty.to_token_stream().to_string().replace(' ', "");
                    if REQUEST_TYPES
                        .iter()
                        .any(|request_type| ty_string.starts_with(request_type))
                    {
                        if req_pat.is_some() {
                            Some(Err(syn::Error::new(ty.span(), "request defined twice")))
                        } else {
                            req_pat = Some((pat.as_ref().clone(), ty.as_ref().clone()));
                            None
                        }
                    } else {
                        Some(Ok((pat.as_ref().clone(), ty.as_ref().clone())))
                    }
                }
            })
            .collect::<Result<_, _>>()?;

        let req_arg: FnArg = syn::parse2(quote! { mut req: ::submillisecond::Request }).unwrap();
        item_fn.sig.inputs = Punctuated::from_iter([req_arg]);

        // Get return type and overwrite it with submillisecond::Response
        let return_ty = match item_fn.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => match ty.as_ref() {
                Type::ImplTrait(_) => {
                    return Err(syn::Error::new(
                        ty.span(),
                        "routes cannot return impl types, use `_` instead",
                    ));
                }
                Type::Infer(_) => None,
                _ => Some(*ty),
            },
        };

        item_fn.sig.output = syn::parse2(quote! { -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> }).unwrap();

        Ok(Route {
            attrs,
            item_fn,
            req_pat,
            extractors,
            return_ty,
        })
    }

    pub fn expand(self) -> TokenStream {
        let Route {
            attrs: RouteAttrs { path },
            req_pat,
            extractors,
            return_ty,
            ..
        } = &self;

        self.expand_with_body(|req, body| {
            let define_req_expanded = match req_pat {
                Some((req_pat, req_ty)) => quote! { let mut #req_pat: #req_ty = #req; },
                None => quote! {},
            };

            let define_extractors_expanded = extractors.iter().map(|(pat, ty)| quote! {
                let #pat = match <#ty as ::submillisecond::extract::FromRequest>::from_request(&mut #req) {
                    Ok(val) => val,
                    Err(err) => return ::std::result::Result::Err(
                        ::submillisecond::router::RouteError::ExtractorError(::submillisecond::response::IntoResponse::into_response(err))
                    ),
                };
            });

            let return_ty_expanded = match return_ty {
                Some(return_ty) => quote! { #return_ty },
                None => quote! { _ },
            };

            quote! {
                {
                    let route = #req.extensions().get::<::submillisecond::router::Route>().unwrap();
                    if !route.matches(#path) {
                        return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(#req));
                    }
                }

                #define_req_expanded
                #( #define_extractors_expanded )*

                let response: #return_ty_expanded = (move || {
                    #body
                })();

                ::std::result::Result::Ok(::submillisecond::response::IntoResponse::into_response(response))
            }
        })
    }

    fn expand_with_body(
        &self,
        f: impl FnOnce(Ident, proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    ) -> TokenStream {
        let Route {
            item_fn:
                ItemFn {
                    attrs,
                    vis,
                    sig,
                    block,
                },
            ..
        } = self;

        let attrs_expanded = if attrs.is_empty() {
            quote! {}
        } else {
            quote! {
                #[#(#attrs)*]
            }
        };

        let stmts = &block.stmts;
        let stmts_expanded = quote! { #( #stmts )* };

        let body = f(format_ident!("req"), stmts_expanded);

        quote! {
            #attrs_expanded
            #vis #sig {
                #body
            }
        }
        .into()
    }
}

#[derive(Debug)]
struct RouteAttrs {
    path: LitStr,
}

impl Parse for RouteAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse().map_err(|mut err| {
            err.extend(syn::Error::new(
                input.span(),
                "missing or invalid route path",
            ));
            err
        })?;

        Ok(RouteAttrs { path })
    }
}
