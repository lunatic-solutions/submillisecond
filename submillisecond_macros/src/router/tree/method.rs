use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

syn::custom_keyword!(GET);
syn::custom_keyword!(POST);
syn::custom_keyword!(PUT);
syn::custom_keyword!(DELETE);
syn::custom_keyword!(HEAD);
syn::custom_keyword!(OPTIONS);
syn::custom_keyword!(PATCH);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Method {
    Get(GET),
    Post(POST),
    Put(PUT),
    Delete(DELETE),
    Head(HEAD),
    Options(OPTIONS),
    Patch(PATCH),
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

impl Spanned for Method {
    fn span(&self) -> proc_macro2::Span {
        match self {
            Method::Get(get) => get.span(),
            Method::Post(post) => post.span(),
            Method::Put(put) => put.span(),
            Method::Delete(delete) => delete.span(),
            Method::Head(head) => head.span(),
            Method::Options(options) => options.span(),
            Method::Patch(patch) => patch.span(),
        }
    }
}
