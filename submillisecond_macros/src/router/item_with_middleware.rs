use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, custom_keyword, token, Expr, Token};

custom_keyword!(with);

#[derive(Clone, Debug)]
pub struct ItemWithMiddleware {
    pub with_token: with,
    pub items: Punctuated<Expr, Token![,]>,
}

impl Parse for ItemWithMiddleware {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let with_token = input.parse()?;
        let items = if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            Punctuated::parse_separated_nonempty(&content)?
        } else {
            let mut items = Punctuated::new();
            items.push(input.parse()?);
            items
        };

        Ok(ItemWithMiddleware { with_token, items })
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;
    use syn::parse_quote;

    use super::ItemWithMiddleware;

    #[test]
    fn item_with_items() {
        let item_use: ItemWithMiddleware = parse_quote! {
            with [global, logger(warn)]
        };
        let items = item_use.items;
        assert_eq!(
            items
                .iter()
                .map(|list| list.to_token_stream().to_string().replace(' ', ""))
                .collect::<Vec<_>>(),
            ["global", "logger(warn)"]
        );
    }
}
