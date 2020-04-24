use asn1rs_model::model::Tag;
use proc_macro2::Delimiter;
use syn::parse::{Parse, ParseBuffer};

pub struct AttrTag(pub Tag);

impl Parse for AttrTag {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Self> {
        input.step(|s| {
            let (group, _span, outer) = s
                .group(Delimiter::Parenthesis)
                .ok_or_else(|| input.error("Expected parenthesis"))?;
            if let Some((variant, cursor)) = group.ident() {
                let (variant_group, _span, _outer) = cursor
                    .group(Delimiter::Parenthesis)
                    .ok_or_else(|| syn::Error::new(cursor.span(), "Expected parenthesis"))?;
                let (number, _cursor) = variant_group.literal().ok_or_else(|| {
                    syn::Error::new(variant_group.span(), "Expected number literal")
                })?;
                let number = number.to_string().parse::<usize>().map_err(|_| {
                    syn::Error::new(variant_group.span(), "Literal is not a number")
                })?;
                Ok((
                    AttrTag(match variant.to_string().to_lowercase().as_str() {
                        "universal" => Tag::Universal(number),
                        "application" => Tag::Application(number),
                        "private" => Tag::Private(number),
                        v => return Err(input.error(format!("Unexpected tag variant `{}`", v))),
                    }),
                    outer,
                ))
            } else if let Some((literal, _cursor)) = group.literal() {
                let number = literal
                    .to_string()
                    .parse::<usize>()
                    .map_err(|_| syn::Error::new(group.span(), "Literal is not a number"))?;
                Ok((AttrTag(Tag::ContextSpecific(number)), outer))
            } else {
                Err(syn::Error::new(group.span(), "Expected tag variant"))
            }
        })
    }
}
