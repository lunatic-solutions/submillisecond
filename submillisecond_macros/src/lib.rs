mod route;

use proc_macro::TokenStream;
use route::Route;

#[proc_macro_attribute]
pub fn route(attr: TokenStream, item: TokenStream) -> TokenStream {
    match Route::parse_with_attributes(attr, item.clone()) {
        Ok(route) => route.expand(),
        Err(err) => TokenStream::from_iter([TokenStream::from(err.into_compile_error()), item]),
    }
}
