use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    Token,
};

use crate::hquote;

use super::item_route::ItemHandler;

/// _ => handler
#[derive(Clone, Debug)]
pub struct ItemCatchAll {
    pub underscore_token: Token![_],
    pub fat_arrow_token: Token![=>],
    pub handler: Box<ItemHandler>,
}

impl ItemCatchAll {
    pub fn expand_catch_all_handler(handler: Option<&ItemHandler>) -> TokenStream {
        match handler {
            Some(handler) => match handler {
                ItemHandler::Expr(handler) => {
                    hquote! {
                        ::submillisecond::Handler::handle(#handler, req)
                    }
                }
                ItemHandler::SubRouter(subrouter) => {
                    let handler = subrouter.expand();

                    hquote! {
                        ::submillisecond::Handler::handle(#handler, req)
                    }
                }
            },
            None => {
                hquote! { ::submillisecond::defaults::err_404() }
            }
        }
    }
}

impl Parse for ItemCatchAll {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ItemCatchAll {
            underscore_token: input.parse()?,
            fat_arrow_token: input.parse()?,
            handler: input.parse()?,
        })
    }
}
