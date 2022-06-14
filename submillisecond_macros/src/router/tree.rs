use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};

pub struct RouterTree {}

impl RouterTree {
    pub fn expand(&self) -> TokenStream {
        quote! {
            ((|mut req: ::submillisecond::Request| -> ::std::result::Result<::submillisecond::Response, ::submillisecond::router::RouteError> {
                todo!()
            }) as ::submillisecond::router::HandlerFn)
        }
        .into()
    }
}

impl Parse for RouterTree {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        todo!()
    }
}
