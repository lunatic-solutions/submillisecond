use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};

syn::custom_keyword!(GET);
syn::custom_keyword!(POST);
syn::custom_keyword!(PUT);
syn::custom_keyword!(DELETE);
syn::custom_keyword!(HEAD);
syn::custom_keyword!(OPTIONS);
syn::custom_keyword!(PATCH);

#[derive(Clone, Copy, Debug)]
pub enum Method {
    Get(GET),
    Post(POST),
    Put(PUT),
    Delete(DELETE),
    Head(HEAD),
    Options(OPTIONS),
    Patch(PATCH),
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get(_) => write!(f, "GET"),
            Method::Post(_) => write!(f, "POST"),
            Method::Put(_) => write!(f, "PUT"),
            Method::Delete(_) => write!(f, "DELETE"),
            Method::Head(_) => write!(f, "HEAD"),
            Method::Options(_) => write!(f, "OPTIONS"),
            Method::Patch(_) => write!(f, "PATCH"),
        }
    }
}

impl Method {
    pub fn peek(input: ParseStream) -> bool {
        input.peek(GET)
            || input.peek(POST)
            || input.peek(PUT)
            || input.peek(DELETE)
            || input.peek(HEAD)
            || input.peek(OPTIONS)
            || input.peek(PATCH)
    }
}

impl Parse for Method {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(GET) {
            return Ok(Method::Get(input.parse()?));
        }
        if input.peek(POST) {
            return Ok(Method::Post(input.parse()?));
        }
        if input.peek(PUT) {
            return Ok(Method::Put(input.parse()?));
        }
        if input.peek(DELETE) {
            return Ok(Method::Delete(input.parse()?));
        }
        if input.peek(HEAD) {
            return Ok(Method::Head(input.parse()?));
        }
        if input.peek(OPTIONS) {
            return Ok(Method::Options(input.parse()?));
        }
        if input.peek(PATCH) {
            return Ok(Method::Patch(input.parse()?));
        }

        Err(input.error(
            "expected http method `GET`, `POST`, `PUT`, `DELETE`, `HEAD`, `OPTIONS`, or `PATCH`",
        ))
    }
}

impl ToTokens for Method {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Method::Get(get) => tokens.extend(quote! { #get }),
            Method::Post(post) => tokens.extend(quote! { #post }),
            Method::Put(put) => tokens.extend(quote! { #put }),
            Method::Delete(delete) => tokens.extend(quote! { #delete }),
            Method::Head(head) => tokens.extend(quote! { #head }),
            Method::Options(options) => tokens.extend(quote! { #options }),
            Method::Patch(patch) => tokens.extend(quote! { #patch }),
        }
    }
}
