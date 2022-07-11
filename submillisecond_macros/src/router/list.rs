use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Path, Token,
};

use crate::hquote;

#[derive(Clone, Debug, Default)]
pub struct RouterList {
    pub handlers: Punctuated<Path, Token![,]>,
}

impl RouterList {
    pub fn expand(&self) -> TokenStream {
        let inner = self.expand_inner(&[]);

        hquote! {
            (|mut req: ::submillisecond::Request,
                mut params: ::submillisecond::params::Params,
                mut reader: ::submillisecond::core::UriReader| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::RouteError> {
                #inner
            }) as ::submillisecond::Router
        }
    }

    pub fn expand_inner(&self, middlewares: &[TokenStream]) -> TokenStream {
        let handlers = self.handlers.iter();
        let handlers_len = self.handlers.len();
        let middlewares_expanded = middlewares.iter().map(|item|
            hquote! {
                ::submillisecond::request_context::inject_middleware(Box::new(<#item as Default>::default()));
           });

        hquote! {
            const HANDLERS: [::submillisecond::Router; #handlers_len] = [
                #( #handlers ),*
            ];
            #( #middlewares_expanded )*

            for handler in HANDLERS {
                match handler(req, params.clone(), reader.clone()) {
                    ::std::result::Result::Ok(resp) => {
                        return ::std::result::Result::Ok(resp)
                    }
                    ::std::result::Result::Err(::submillisecond::RouteError::ExtractorError(resp)) =>
                        return ::std::result::Result::Err(::submillisecond::RouteError::ExtractorError(resp)),
                    ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(request)) => req = request,
                }
            }

            return ::std::result::Result::Err(::submillisecond::RouteError::RouteNotMatch(req));
        }
    }
}

impl Parse for RouterList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(RouterList {
            handlers: Punctuated::parse_terminated(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::RouterList;

    #[test]
    fn parse_router_list() {
        let router_list: RouterList = parse_quote!(a, b, c);
        assert_eq!(
            router_list.handlers.to_token_stream().to_string(),
            "a , b , c"
        );

        let router_list: RouterList = parse_quote!(a::b::c, d::e::f, g::h::i);
        assert_eq!(
            router_list.handlers.to_token_stream().to_string(),
            "a :: b :: c , d :: e :: f , g :: h :: i"
        );
    }
}
