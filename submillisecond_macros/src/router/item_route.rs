use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, Expr, LitStr, Path, Token,
};

use crate::{hquote, router::Router};

use super::{item_with_middleware::ItemUseMiddleware, method::Method, with};

/// `"/abc" => sub_router`
/// `GET "/abc" => handler`
/// `GET "/abc" if guard => handler`
/// `GET "/abc" use middleware => handler`
/// `GET "/abc" if guard use middleware => handler`
#[derive(Clone, Debug)]
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
            middleware: if input.peek(with) {
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

#[derive(Clone, Debug)]
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

impl ToTokens for ItemGuard {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(expand_guard_struct(&self.guard));
    }
}

fn expand_guard_struct(guard: &syn::Expr) -> TokenStream {
    match guard {
        Expr::Binary(expr_binary) => {
            let left = expand_guard_struct(&expr_binary.left);
            let op = &expr_binary.op;
            let right = expand_guard_struct(&expr_binary.right);

            hquote! { #left #op #right }
        }
        Expr::Paren(expr_paren) => {
            let expr = expand_guard_struct(&expr_paren.expr);
            hquote! { (#expr) }
        }
        expr => hquote! { ::submillisecond::guard::Guard::check(&#expr, &req) },
    }
}

#[derive(Clone, Debug)]
pub enum ItemHandler {
    Expr(Box<Expr>),
    SubRouter(Router),
}

impl Parse for ItemHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            return Ok(ItemHandler::SubRouter(content.parse()?));
        }

        let fork = input.fork();
        let _: Path = fork.parse()?;
        Ok(ItemHandler::Expr(input.parse()?))
    }
}
