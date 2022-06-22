use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Ident, Index, LitStr};

#[derive(Debug, Eq, PartialEq, FromAttributes)]
#[bae("param")]
pub struct Attributes {
    name: LitStr,
}

#[derive(Debug)]
pub struct NamedParam {
    ident: Ident,
    fields: NamedParamFields,
}

impl NamedParam {
    pub fn expand(&self) -> TokenStream {
        let NamedParam { ident, fields } = self;

        let content = match fields {
            NamedParamFields::Named(named) => {
                let names = named.iter().map(|NamedParamField { name, .. }| name);
                let tuple_types = named.iter().map(|_| quote! { _ });
                let fields = named
                    .iter()
                    .enumerate()
                    .map(|(i, NamedParamField { ident, .. })| {
                        let index = Index::from(i);
                        quote! {
                            #ident: params.#index
                        }
                    });

                quote! {
                    let params = req
                        .extensions()
                        .get::<::submillisecond_core::router::params::Params>()
                        .unwrap();

                    let fields = ::std::iter::Iterator::collect::<::std::result::Result<::std::vec::Vec<_>, _>>(
                        ::std::iter::Iterator::map(
                            ::std::iter::IntoIterator::into_iter([#( #names ),*]),
                            |name| {
                                let value = params
                                    .get(name)
                                    .ok_or_else(<::submillisecond::extract::rejection::MissingPathParams as ::std::default::Default>::default)?;

                                let percent_decoded_str = ::submillisecond::extract::path::de::PercentDecodedStr::new(value)
                                    .ok_or_else(|| {
                                        ::submillisecond::extract::rejection::PathRejection::FailedToDeserializePathParams(
                                            ::submillisecond::extract::path::FailedToDeserializePathParams(
                                                ::submillisecond::extract::path::PathDeserializationError::new(
                                                    ::submillisecond::extract::path::ErrorKind::InvalidUtf8InPathParam {
                                                        key: ::std::string::ToString::to_string(name),
                                                    },
                                                ),
                                            ),
                                        )
                                    })?;

                                ::std::result::Result::<_, ::submillisecond::extract::rejection::PathRejection>::Ok(
                                    (::std::convert::From::from(name), percent_decoded_str)
                                )
                            },
                        ),
                    )?;

                    ::serde::de::Deserialize::deserialize(
                        ::submillisecond::extract::path::de::PathDeserializer::new(fields.as_slice()),
                    )
                    .map_err(|err| {
                        ::submillisecond::extract::rejection::PathRejection::FailedToDeserializePathParams(
                            ::submillisecond::extract::path::FailedToDeserializePathParams(err),
                        )
                    })
                    .map(|params: (#( #tuple_types ),*)| #ident {
                        #( #fields ),*
                    })
                }
            }
            NamedParamFields::Unnamed(NamedParamField { name, .. }) => {
                quote! {
                    let param_str = req
                        .extensions()
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
        };

        quote! {
            impl ::submillisecond::extract::FromRequest for #ident {
                type Rejection = ::submillisecond::extract::rejection::PathRejection;

                fn from_request(
                    req: &mut ::submillisecond::Request,
                ) -> ::std::result::Result<Self, Self::Rejection> {
                    #content
                }
            }
        }
    }
}

impl TryFrom<DeriveInput> for NamedParam {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> syn::Result<Self> {
        let span = input.span();
        let fields = match input.data {
            Data::Enum(_) => {
                return Err(syn::Error::new(
                    span,
                    "enum is not supported with NamedParam",
                ))
            }
            Data::Struct(data_struct) => match data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    let attrs = Attributes::try_from_attributes(&input.attrs)?;
                    if let Some(attrs) = attrs {
                        return Err(syn::Error::new(
                            attrs.name.span(),
                            "Param name can only be applied to unnamed structs with a single value. You might have meant to place it above a field instead?",
                        ));
                    }

                    let fields = fields_named
                        .named
                        .into_iter()
                        .map(|named| {
                            let name = Attributes::try_from_attributes(&named.attrs)?
                                .map(|attrs| attrs.name.value())
                                .unwrap_or_else(|| named.ident.clone().unwrap().to_string());

                            syn::Result::Ok(NamedParamField {
                                name,
                                ident: named.ident,
                            })
                        })
                        .collect::<Result<_, _>>()?;

                    NamedParamFields::Named(fields)
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    let fields_unnamed_span = fields_unnamed.span();
                    let mut fields_iter = fields_unnamed.unnamed.into_iter();
                    fields_iter.next().ok_or_else(|| {
                        syn::Error::new(fields_unnamed_span, "expected unnamed field")
                    })?;
                    if let Some(field) = fields_iter.next() {
                        return Err(syn::Error::new(
                            field.span(),
                            "only one field can be used with NamedParam",
                        ));
                    }

                    let name = Attributes::from_attributes(&input.attrs)?.name.value();
                    NamedParamFields::Unnamed(NamedParamField { name, ident: None })
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
            ident: input.ident,
            fields,
        })
    }
}

#[derive(Debug)]
enum NamedParamFields {
    Named(Vec<NamedParamField>),
    Unnamed(NamedParamField),
}

#[derive(Debug)]
struct NamedParamField {
    name: String,
    ident: Option<Ident>,
}
