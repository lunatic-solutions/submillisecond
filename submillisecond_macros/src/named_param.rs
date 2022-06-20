use better_bae::{FromAttributes, TryFromAttributes};
use proc_macro2::TokenStream;
use quote::{__private::ext::RepToTokensExt, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Field, Ident, LitStr};

#[derive(Debug, Eq, PartialEq, FromAttributes)]
#[bae("param")]
pub struct Attributes {
    name: LitStr,
}

#[derive(Debug)]
pub struct NamedParam {
    attrs: Attributes,
    ident: Ident,
    field: Field,
}

impl NamedParam {
    pub fn expand(&self) -> TokenStream {
        let NamedParam {
            attrs: Attributes { name },
            ident,
            field:
                Field {
                    ident: field_ident,
                    ty: field_ty,
                    ..
                },
        } = self;

        quote! {
            impl ::submillisecond::extract::FromRequest for #ident {
                type Rejection = ::submillisecond::extract::rejection::PathRejection;

                fn from_request(req: &mut ::submillisecond::Request) -> ::std::result::Result<Self, Self::Rejection> {
                    let param = req
                        .extensions_mut()
                        .get::<::submillisecond_core::router::params::Params>()
                        .unwrap()
                        .get(#name)
                        .ok_or(::submillisecond::extract::rejection::PathRejection::MissingPathParams)?
                        .map(|v| {
                            if let Some(decoded) = ::extract::path::de::PercentDecodedStr::new(v) {
                                ::std::result::Result::Ok(decoded)
                            } else {
                                ::std::result::Result::Err(PathRejection::FailedToDeserializePathParams(
                                    FailedToDeserializePathParams(PathDeserializationError {
                                        kind: ErrorKind::InvalidUtf8InPathParam { key: #name.to_string() },
                                    }),
                                ))
                            }
                        })?;

                    T::deserialize(::extract::path::de::PathDeserializer::new(&*params))
                        .map_err(|err| {
                            PathRejection::FailedToDeserializePathParams(FailedToDeserializePathParams(err))
                        })
                        .map(Path)
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
        let field = match input.data {
            Data::Enum(data_enum) => {
                return Err(syn::Error::new(
                    span,
                    "enum is not supported with NamedParam",
                ))
            }
            Data::Struct(data_struct) => match data_struct.fields {
                syn::Fields::Named(fields_named) => {
                    return Err(syn::Error::new(
                        span,
                        "struct with named fields is not supported with NamedParam",
                    ))
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    let fields_unnamed_span = fields_unnamed.span();
                    let mut fields_iter = fields_unnamed.unnamed.into_iter();
                    let field = fields_iter.next().ok_or_else(|| {
                        syn::Error::new(fields_unnamed_span, "expected unnamed field")
                    })?;
                    if let Some(field) = field.next() {
                        return Err(syn::Error::new(
                            field.span(),
                            "only one field can be used with NamedParam",
                        ));
                    }

                    field
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
            field,
        })
    }
}
