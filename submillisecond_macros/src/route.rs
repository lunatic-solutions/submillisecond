use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    FnArg, Ident, ItemFn, LitStr, Pat, PatType, ReturnType, Type,
};

pub struct Route {
    attrs: RouteAttrs,
    item_fn: ItemFn,
    req_pat: Option<Pat>,
    return_ty: Option<Type>,
}

impl Route {
    pub fn parse_with_attributes(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        let attrs = syn::parse(attr)?;
        let mut item_fn: ItemFn = syn::parse(item)?;

        // Get request param and overwrite it with submillisecond::Request
        let req_pat = item_fn
            .sig
            .inputs
            .first()
            .map(|input| match input {
                FnArg::Receiver(_) => Err(syn::Error::new(input.span(), "routes cannot take self")),
                FnArg::Typed(PatType { pat, .. }) => Ok(pat.as_ref().clone()),
            })
            .transpose()?;

        if item_fn.sig.inputs.len() > 1 {
            return Err(syn::Error::new(
                item_fn.sig.inputs.span(),
                "routes cannot take more than one parameter",
            ));
        }

        let req_arg: FnArg =
            syn::parse2(quote! { req: ::submillisecond::Request<String> }).unwrap();
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

        item_fn.sig.output = syn::parse2(quote! { -> ::submillisecond::Response }).unwrap();

        Ok(Route {
            attrs,
            item_fn,
            req_pat,
            return_ty,
        })
    }

    pub fn expand(self) -> TokenStream {
        let Route {
            attrs: RouteAttrs { path },
            req_pat,
            return_ty,
            ..
        } = &self;

        self.expand_with_body(|req, body| {
            let define_req_expanded = match req_pat {
                Some(req_pat) => quote! { let #req_pat = #req; },
                None => quote! {},
            };

            let return_ty_expanded = match return_ty {
                Some(return_ty) => quote! { #return_ty },
                None => quote! { _ },
            };

            quote! {
                {
                    println!("--------------------");
                    println!("Expected Route: {}", #path);
                    println!("Received Route: {}", #req.uri().path());
                    println!("--------------------");
                }

                let response: #return_ty_expanded = {
                    #define_req_expanded
                    #body
                };

                ::submillisecond::response::IntoResponse::into_response(response)
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
