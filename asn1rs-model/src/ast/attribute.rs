use super::range::ident_or_literal_or_punct;
use super::range::IntegerRange;
use super::tag::AttrTag;
use crate::ast::constants::ConstLit;
use crate::model::{
    Charset, Choice, ChoiceVariant, Enumerated, EnumeratedVariant, Range, Size, Tag, Type,
};
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::Deref;
use std::str::FromStr;
use syn::parenthesized;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::token;
use syn::Token;

#[derive(Debug)]
pub(crate) struct AsnAttribute<C: Context> {
    pub(crate) primary: C::Primary,
    pub(crate) tag: Option<Tag>,
    pub(crate) consts: Vec<ConstLit>,
    pub(crate) extensible_after: Option<String>,
    _c: PhantomData<C>,
}

impl<C: Context> AsnAttribute<C> {
    pub fn new(primary: C::Primary) -> Self {
        Self {
            primary,
            tag: None,
            consts: Vec::default(),
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
                "extensible_after" if C::EXTENSIBLE_AFTER => {
                    let content;
                    parenthesized!(content in input);
                    let ident = content
                        .step(|s| s.ident().ok_or_else(|| content.error("Not a valid ident")))?;
                    asn.extensible_after = Some(ident.to_string());
                }
                "const" if C::CONSTS => {
                    let content;
                    parenthesized!(content in input);
                    loop {
                        asn.consts.push(content.parse::<ConstLit>()?);
                        if content.is_empty() {
                            break;
                        }
                        let _ = content.parse::<token::Comma>()?;
                    }
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
        // "utf8string" => parse_opt_size_or_any(input).map(|size| Type::String(size, Charset::Utf8)),
        // "ia5string" => parse_opt_size_or_any(input).map(|size| Type::String(size, Charset::Ia5)),
        "octet_string" => parse_opt_size_or_any(input).map(Type::OctetString),
        "bit_string" => parse_opt_size_or_any(input).map(Type::bit_vec_with_size),
        string if string.ends_with("string") => {
            let len = string.chars().count();
            let charset = &string[..len - "string".chars().count()];
            let charset = Charset::from_str(&charset)
                .map_err(|_| input.error(format!("Unexpected charset '{}'", charset)))?;
            parse_opt_size_or_any(input).map(|size| Type::String(size, charset))
        }
        "integer" => {
            if input.is_empty() {
                Ok(Type::unconstrained_integer())
            } else {
                let content;
                parenthesized!(content in input);
                if content.is_empty() {
                    Ok(Type::unconstrained_integer())
                } else {
                    let int_range = IntegerRange::parse(&content)?;
                    Ok(Type::integer_with_range_opt(
                        int_range
                            .0
                            .map(|(min, max)| Range::inclusive(Some(min), Some(max)))
                            .unwrap_or_else(Range::none)
                            .with_extensible(int_range.1),
                    ))
                }
            }
        }
        "complex" => {
            let content;
            parenthesized!(content in input);
            let ident: syn::Ident = content.parse()?;
            let _ = content.parse::<Token![,]>()?;
            let tag_ident: syn::Ident = content.parse()?;
            if !"tag".eq_ignore_ascii_case(&tag_ident.to_string()) {
                return Err(input.error("Expected identifier 'tag'"));
            }
            let tag = AttrTag::parse(&content)?;
            Ok(Type::TypeReference(ident.to_string(), Some(tag.0)))
        }
        "option" | "optional" => {
            let content;
            parenthesized!(content in input);
            let inner = parse_type(&content)?;
            Ok(Type::Optional(Box::new(inner)))
        }
        "boolean" => Ok(Type::Boolean),
        "sequence_of" | "set_of" => {
            let content;
            parenthesized!(content in input);
            let size = if content.peek2(Token![.])
                || (content.peek(Token![-]) && content.peek3(Token![.]))
            {
                let size = Size::parse(&content)?;
                let _ = content.parse::<token::Comma>()?;
                size
            } else {
                Size::Any
            };
            let inner = parse_type(&content)?;
            if lowercase_ident == "sequence_of" {
                Ok(Type::SequenceOf(Box::new(inner), size))
            } else {
                // "set_of"
                Ok(Type::SetOf(Box::new(inner), size))
            }
        }
        r#type => Err(input.error(format!("Unexpected attribute: `{}`", r#type))),
    }
}

fn parse_opt_size_or_any(input: ParseStream) -> syn::Result<Size> {
    if input.is_empty() || !input.peek(token::Paren) {
        Ok(Size::Any)
    } else {
        let content;
        parenthesized!(content in input);
        if content.is_empty() {
            Ok(Size::Any)
        } else {
            Size::parse(&content)
        }
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
    const EXTENSIBLE_AFTER: bool;
    const TAGGABLE: bool;
    const CONSTS: bool;
}

impl Context for Choice {
    type Primary = Type;
    const EXTENSIBLE_AFTER: bool = true;
    const TAGGABLE: bool = true;
    const CONSTS: bool = false;
}

impl Context for ChoiceVariant {
    type Primary = Type;
    const EXTENSIBLE_AFTER: bool = false;
    const TAGGABLE: bool = true;
    const CONSTS: bool = false;
}

impl Context for Enumerated {
    type Primary = Type;
    const EXTENSIBLE_AFTER: bool = true;
    const TAGGABLE: bool = true;
    const CONSTS: bool = false;
}

impl Context for EnumeratedVariant {
    type Primary = Option<usize>;
    const EXTENSIBLE_AFTER: bool = false;
    const TAGGABLE: bool = false;
    const CONSTS: bool = false;
}

pub struct Transparent;
impl Context for Transparent {
    type Primary = Type;
    const EXTENSIBLE_AFTER: bool = false;
    const TAGGABLE: bool = true;
    const CONSTS: bool = true;
}

pub struct DefinitionHeader(String);
impl Context for DefinitionHeader {
    type Primary = Self;
    const EXTENSIBLE_AFTER: bool = true;
    const TAGGABLE: bool = true;
    const CONSTS: bool = false;
}

impl Deref for DefinitionHeader {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl PrimaryContext for DefinitionHeader {
    fn parse(input: &ParseBuffer<'_>) -> syn::Result<Self> {
        input
            .step(|c| c.ident().ok_or_else(|| c.error("Expected type identifier")))
            .map(|ident| ident.to_string())
            .map(DefinitionHeader)
    }
}
