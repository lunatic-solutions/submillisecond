use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Path, Token,
};

#[derive(Debug, Default)]
pub struct RouterList {
    pub handlers: Punctuated<Path, Token![,]>,
}

impl RouterList {
    pub fn expand(&self) -> TokenStream {
        let inner = self.expand_inner(&[]);

        quote! {
            (|mut __req: ::submillisecond::Request,
                mut __params: ::submillisecond::params::Params,
                mut __reader: ::submillisecond::core::UriReader| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                #inner
            }) as ::submillisecond::handler::HandlerFn
        }
    }

    pub fn expand_inner(&self, middlewares: &[TokenStream]) -> TokenStream {
        let handlers = self.handlers.iter();
        let handlers_len = self.handlers.len();
        let middlewares_expanded = middlewares.iter().map(|item|
            quote! {
                ::submillisecond::request_context::inject_middleware(Box::new(<#item as Default>::default()));
           });

        quote! {
            const HANDLERS: [::submillisecond::handler::HandlerFn; #handlers_len] = [
                #( #handlers ),*
            ];
            #( #middlewares_expanded )*

            for handler in HANDLERS {
                match handler(__req, __params.clone(), __reader.clone()) {
                    ::std::result::Result::Ok(__resp) => {
                        return ::std::result::Result::Ok(__resp)
                    }
                    ::std::result::Result::Err(::submillisecond::router::RouteError::ExtractorError(resp)) =>
                        return ::std::result::Result::Err(::submillisecond::router::RouteError::ExtractorError(resp)),
                    ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request)) => __req = request,
                }
            }

            return ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(__req));
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
