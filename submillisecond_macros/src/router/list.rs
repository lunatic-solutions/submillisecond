use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Path, Token,
};

#[derive(Debug, Default)]
pub struct RouterList {
    handlers: Punctuated<Path, Token![,]>,
}

impl RouterList {
    pub fn expand(&self) -> TokenStream {
        let handlers = self.handlers.iter();
        let handlers_len = self.handlers.len();

        quote! {
            ((|mut req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                const HANDLERS: [::submillisecond::router::HandlerFn; #handlers_len] = [
                    #( #handlers ),*
                ];

                for handler in HANDLERS {
                    match handler(req) {
                        ::std::result::Result::Ok(resp) => return ::std::result::Result::Ok(resp),
                        ::std::result::Result::Err(::submillisecond::router::RouteError::ExtractorError(resp)) =>
                            return ::std::result::Result::Err(::submillisecond::router::RouteError::ExtractorError(resp)),
                        ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(request)) => req = request,
                    }
                }

                ::std::result::Result::Err(::submillisecond::router::RouteError::RouteNotMatch(req))
            }) as ::submillisecond::router::HandlerFn)
        }
        .into()
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
