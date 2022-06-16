use syn::{
    parse::{Parse, ParseStream},
    token, Expr, LitStr, Path, Token,
};

use crate::router::Router;

use super::{item_use_middleware::ItemUseMiddleware, method::Method};

/// `"/abc" => sub_router`
/// `GET "/abc" => handler`
/// `GET "/abc" if guard => handler`
/// `GET "/abc" use middleware => handler`
/// `GET "/abc" if guard use middleware => handler`
#[derive(Debug)]
pub struct ItemRoute {
    pub method: Option<Method>,
    pub path: LitStr,
    pub guard: Option<ItemGuard>,
    pub middleware: Option<ItemUseMiddleware>,
    pub fat_arrow_token: Token![=>],
    pub handler: ItemHandler,
}

impl Parse for ItemRoute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ItemRoute {
            method: if Method::peek(input) {
                Some(input.parse()?)
            } else {
                None
            },
            path: input.parse()?,
            guard: if input.peek(Token![if]) {
                Some(input.parse()?)
            } else {
                None
            },
            middleware: if input.peek(Token![use]) {
                Some(input.parse()?)
            } else {
                None
            },
            fat_arrow_token: input.parse()?,
            handler: input.parse()?,
        })
    }
}

#[derive(Debug)]
pub struct ItemGuard {
    pub if_token: Token![if],
    pub guard: Box<Expr>,
}

impl Parse for ItemGuard {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ItemGuard {
            if_token: input.parse()?,
            guard: Box::new(input.call(Expr::parse_without_eager_brace)?),
        })
    }
}

#[derive(Debug)]
pub enum ItemHandler {
    Fn(Path),
    SubRouter(Router),
}

impl Parse for ItemHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) || input.peek(token::Bracket) {
            return Ok(ItemHandler::SubRouter(input.parse()?));
        }

        Ok(ItemHandler::Fn(input.parse()?))
    }
}
