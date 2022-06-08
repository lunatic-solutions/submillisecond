mod route;

use proc_macro::TokenStream;
use route::{Route, RouteMethod};

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::GET, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}

#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::POST, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}
