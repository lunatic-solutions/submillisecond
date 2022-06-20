use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Ident, LitStr};

#[derive(Debug, Eq, PartialEq, FromAttributes)]
#[bae("param")]
pub struct Attributes {
    name: LitStr,
}

#[derive(Debug)]
pub struct NamedParam {
    attrs: Attributes,
    ident: Ident,
}

impl NamedParam {
    pub fn expand(&self) -> TokenStream {
        let NamedParam {
            attrs: Attributes { name },
            ident,
        } = self;

        quote! {
            impl ::submillisecond::extract::FromRequest for #ident {
                type Rejection = ::submillisecond::extract::rejection::PathRejection;

                fn from_request(
                    req: &mut ::submillisecond::Request,
                ) -> ::std::result::Result<Self, Self::Rejection> {
                    let param_str = req
                        .extensions_mut()
                        .get::<::submillisecond_core::router::params::Params>()
                        .unwrap()
                        .get(#name)
                        .ok_or_else(<::submillisecond::extract::rejection::MissingPathParams as ::std::default::Default>::default)?;

                    let param = ::submillisecond::extract::path::de::PercentDecodedStr::new(param_str)
                        .ok_or_else(|| {
                            ::submillisecond::extract::rejection::PathRejection::FailedToDeserializePathParams(
                                ::submillisecond::extract::path::FailedToDeserializePathParams(
                                    ::submillisecond::extract::path::PathDeserializationError::new(
                                        ::submillisecond::extract::path::ErrorKind::InvalidUtf8InPathParam {
                                            key: ::std::string::ToString::to_string(#name),
                                        },
                                    ),
                                ),
                            )
                        })?;

                    ::serde::de::Deserialize::deserialize(
                        ::submillisecond::extract::path::de::PathDeserializer::new(&[(
                            ::std::convert::From::from(#name),
                            param,
                        )]),
                    )
                    .map_err(|err| {
                        ::submillisecond::extract::rejection::PathRejection::FailedToDeserializePathParams(
                            ::submillisecond::extract::path::FailedToDeserializePathParams(err),
                        )
                    })
                    .map(Self)
                }
            }
        }
    }
}

impl TryFrom<DeriveInput> for NamedParam {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> syn::Result<Self> {
        let attrs = Attributes::from_attributes(&input.attrs)?;

        let span = input.span();
        let _field = match input.data {
            Data::Enum(_) => {
                return Err(syn::Error::new(
                    span,
                    "enum is not supported with NamedParam",
                ))
            }
            Data::Struct(data_struct) => match data_struct.fields {
                syn::Fields::Named(_fields_named) => {
                    return Err(syn::Error::new(
                        span,
                        "struct with named fields is not supported with NamedParam",
                    ))
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    let fields_unnamed_span = fields_unnamed.span();
                    let mut fields_iter = fields_unnamed.unnamed.into_iter();
                    let _field = fields_iter.next().ok_or_else(|| {
                        syn::Error::new(fields_unnamed_span, "expected unnamed field")
                    })?;
                    if let Some(field) = fields_iter.next() {
                        return Err(syn::Error::new(
                            field.span(),
                            "only one field can be used with NamedParam",
                        ));
                    }
                }
                syn::Fields::Unit => {
                    return Err(syn::Error::new(
                        span,
                        "unit struct is not supported with NamedParam",
                    ))
                }
            },
            Data::Union(_) => {
                return Err(syn::Error::new(
                    span,
                    "union is not supported with NamedParam",
                ))
            }
        };

        Ok(NamedParam {
            attrs,
            ident: input.ident,
        })
    }
}
