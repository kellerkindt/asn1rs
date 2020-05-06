use super::range::ident_or_literal_or_punct;
use super::range::MaybeRanged;
use super::tag::AttrTag;
use crate::model::{Range, Tag, Type};
use syn::parenthesized;
use syn::parse::{Parse, ParseBuffer};

#[derive(Debug, Default)]
pub(crate) struct AsnAttribute {
    pub(crate) r#type: Option<Type>,
    pub(crate) tag: Option<Tag>,
    pub(crate) number: Option<usize>,
}

impl Parse for AsnAttribute {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Self> {
        let mut asn = Self::default();

        while !input.cursor().eof() {
            let lowercase_ident = input
                .step(|c| {
                    ident_or_literal_or_punct(*c).ok_or_else(|| c.error("Expected type or number"))
                })?
                .to_string()
                .to_lowercase();

            match lowercase_ident.as_str() {
                // variant number
                _ if asn.number.is_none() && lowercase_ident.chars().all(char::is_numeric) => {
                    asn.number = Some(
                        lowercase_ident
                            .parse()
                            .map_err(|e| input.error(format!("Invalid number: {}", e)))?,
                    );
                }
                // field type
                _ if asn.r#type.is_none() => {
                    asn.r#type = Some(parse_type_pre_stepped(&lowercase_ident, input)?);
                }
                // --- additional properties
                "tag" => {
                    let tag = AttrTag::parse(input)?;
                    asn.tag = Some(tag.0);
                }
                attribute => {
                    return Err(
                        input.error(format!("Unexpected or repeated attribute: `{}`", attribute))
                    );
                }
            }

            if !input.cursor().eof() && !input.peek(syn::token::Comma) {
                return Err(input.error("Attributes must be separated by comma"));
            } else if !input.cursor().eof() {
                let _ = input.step(|c| {
                    c.punct()
                        .ok_or_else(|| input.error("Attributes must be separated by comma"))
                })?;
            }
        }
        Ok(asn)
    }
}

fn parse_type<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Type> {
    let ident = input
        .step(|c| c.ident().ok_or_else(|| c.error("Expected ASN-Type")))?
        .to_string()
        .to_lowercase();
    parse_type_pre_stepped(&ident, input)
}

fn parse_type_pre_stepped<'a>(
    lowercase_ident: &str,
    input: &'a ParseBuffer<'a>,
) -> syn::Result<Type> {
    match lowercase_ident {
        "utf8string" => Ok(Type::UTF8String),
        "octet_string" => Ok(Type::OctetString),
        "integer" => {
            let range = MaybeRanged::parse(input)?;
            Ok(Type::Integer(range.0.map(|(min, max)| Range(min, max))))
        }
        "complex" => {
            let content;
            parenthesized!(content in input);
            let ident =
                content.step(|c| c.ident().ok_or_else(|| c.error("Expected type identifier")))?;
            Ok(Type::TypeReference(ident.to_string()))
        }
        "option" => {
            let content;
            parenthesized!(content in input);
            let inner = parse_type(&content)?;
            Ok(Type::Optional(Box::new(inner)))
        }
        "boolean" => Ok(Type::Boolean),
        "sequence_of" => {
            let content;
            parenthesized!(content in input);
            let inner = parse_type(&content)?;
            Ok(Type::SequenceOf(Box::new(inner)))
        }
        r#type => Err(input.error(format!("Unexpected attribute: `{}`", r#type))),
    }
}
