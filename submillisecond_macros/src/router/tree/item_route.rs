use syn::{
    braced, bracketed,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, Expr, LitStr, Macro, Path, Token,
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
        let item_route = ItemRoute {
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
        };

        if let Some(method) = item_route.method {
            if matches!(item_route.handler, ItemHandler::SubRouter(_)) {
                return Err(syn::Error::new(
                    method.span(),
                    "method prefix cannot be used with sub router",
                ));
            }
        }

        Ok(item_route)
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
    Macro(Macro),
    SubRouter(Router),
}

impl Parse for ItemHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            return Ok(ItemHandler::SubRouter(Router::Tree(content.parse()?)));
        }

        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            return Ok(ItemHandler::SubRouter(Router::List(content.parse()?)));
        }

        let fork = input.fork();
        let _: Path = fork.parse()?;
        if fork.peek(Token![!]) {
            Ok(ItemHandler::Macro(input.parse()?))
        } else {
            Ok(ItemHandler::Fn(input.parse()?))
        }
    }
}
