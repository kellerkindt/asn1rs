use super::range::ident_or_literal_or_punct;
use super::range::MaybeRanged;
use super::tag::AttrTag;
use crate::model::{Choice, ChoiceVariant, Enumerated, EnumeratedVariant, Range, Tag, Type};
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use syn::parenthesized;
use syn::parse::{Parse, ParseBuffer};

#[derive(Debug)]
pub(crate) struct AsnAttribute<C: Context> {
    pub(crate) primary: C::Primary,
    pub(crate) tag: Option<Tag>,
    pub(crate) extensible_after: Option<String>,
    _c: PhantomData<C>,
}

impl<C: Context> AsnAttribute<C> {
    pub fn new(primary: C::Primary) -> Self {
        Self {
            primary,
            tag: None,
            extensible_after: None,
            _c: Default::default(),
        }
    }
}

impl<C: Context> Parse for AsnAttribute<C> {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Self> {
        let mut asn = Self::new(C::Primary::parse(input)?);
        eof_or_comma(&input, "Primary attribute must be separated by comma")?;

        while !input.cursor().eof() {
            let lowercase_ident = input
                .step(|c| {
                    ident_or_literal_or_punct(*c).ok_or_else(|| c.error("Expected type or number"))
                })?
                .to_string()
                .to_lowercase();

            match lowercase_ident.as_str() {
                "tag" if C::TAGGABLE => {
                    let tag = AttrTag::parse(input)?;
                    asn.tag = Some(tag.0);
                }
                "extensible_after" if C::EXTENSIBLE => {
                    let content;
                    parenthesized!(content in input);
                    let ident = content
                        .step(|s| s.ident().ok_or_else(|| content.error("Not a valid ident")))?;
                    asn.extensible_after = Some(ident.to_string());
                }
                attribute => {
                    return Err(
                        input.error(format!("Unexpected or repeated attribute: `{}`", attribute))
                    );
                }
            }

            eof_or_comma(input, "Attributes must be separated by comma")?;
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

fn eof_or_comma<T: Display>(input: &ParseBuffer, msg: T) -> syn::Result<()> {
    if !input.cursor().eof() && !input.peek(syn::token::Comma) {
        Err(input.error(msg))
    } else if !input.cursor().eof() {
        // skip the comma
        input
            .step(|c| c.punct().ok_or_else(|| input.error(msg)))
            .map(drop)
    } else {
        // eof
        Ok(())
    }
}

pub trait PrimaryContext: Sized {
    fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self>;
}

impl PrimaryContext for Type {
    fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
        let lowercase_ident = input
            .step(|c| {
                ident_or_literal_or_punct(*c)
                    .ok_or_else(|| c.error("Expected type, number or extension marker"))
            })?
            .to_lowercase();

        Ok(parse_type_pre_stepped(&lowercase_ident, input)?)
    }
}

impl PrimaryContext for Option<usize> {
    fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
        input
            .step(|c| {
                ident_or_literal_or_punct(*c)
                    .ok_or_else(|| c.error("Expected type, number or extension marker"))
            })
            .ok()
            .as_ref()
            .map(ToString::to_string)
            .as_deref()
            .map(str::to_lowercase)
            .map(|lowercase_ident| {
                lowercase_ident
                    .parse()
                    .map_err(|e| input.error(format!("Invalid number: {}", e)))
            })
            .transpose()
    }
}

pub trait Context {
    type Primary: PrimaryContext;
    const EXTENSIBLE: bool;
    const TAGGABLE: bool;
}

impl Context for Choice {
    type Primary = Type;
    const EXTENSIBLE: bool = true;
    const TAGGABLE: bool = true;
}

impl Context for ChoiceVariant {
    type Primary = Type;
    const EXTENSIBLE: bool = false;
    const TAGGABLE: bool = true;
}

impl Context for Enumerated {
    type Primary = Type;
    const EXTENSIBLE: bool = true;
    const TAGGABLE: bool = true;
}

impl Context for EnumeratedVariant {
    type Primary = Option<usize>;
    const EXTENSIBLE: bool = false;
    const TAGGABLE: bool = false;
}

pub struct Transparent;
impl Context for Transparent {
    type Primary = Type;
    const EXTENSIBLE: bool = false;
    const TAGGABLE: bool = true;
}

pub struct DefinitionHeader(String);

impl Deref for DefinitionHeader {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context for DefinitionHeader {
    type Primary = Self;
    const EXTENSIBLE: bool = true;
    const TAGGABLE: bool = true;
}

impl PrimaryContext for DefinitionHeader {
    fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
        input
            .step(|c| c.ident().ok_or_else(|| c.error("Expected type identifier")))
            .map(|ident| ident.to_string())
            .map(DefinitionHeader)
    }
}
