use syn::{
    parse::{Parse, ParseStream},
    Token,
};

use super::item_route::ItemHandler;

/// _ => handler
#[derive(Clone, Debug)]
pub struct ItemCatchAll {
    pub underscore_token: Token![_],
    pub fat_arrow_token: Token![=>],
    pub handler: Box<ItemHandler>,
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
