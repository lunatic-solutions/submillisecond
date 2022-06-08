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

#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::PUT, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}

#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::DELETE, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}

#[proc_macro_attribute]
pub fn head(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::HEAD, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}

#[proc_macro_attribute]
pub fn options(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::OPTIONS, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}

#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(RouteMethod::PATCH, attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}
