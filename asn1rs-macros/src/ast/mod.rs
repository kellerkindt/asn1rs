mod range;
mod tag;

use asn1rs_model::model::{
    Choice, ChoiceVariant, Definition, Enumerated, Field, Model, Range, Tag, Type,
};
use proc_macro::TokenStream;
use quote::quote;
use range::MaybeRanged;
use std::str::FromStr;
use syn::export::TokenStream2;
use syn::parse::{Parse, ParseBuffer};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, Meta};
use syn::{Item, NestedMeta};
use tag::AttrTag;

pub(crate) fn parse(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("Attribute: {}", attr.to_string());
    println!("Item:      {}", item.to_string());

    let attributes = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    let asn_type_decl = match attributes.get(0) {
        None => panic!("Missing ASN attribute"),
        Some(NestedMeta::Meta(Meta::Path(path))) => path
            .segments
            .iter()
            .next()
            .expect("Missing ASN Attribute in path")
            .ident
            .to_string()
            .to_lowercase(),
        _ => panic!("Invalid ASN Attribute type"),
    };

    let mut additional_impl: Vec<TokenStream2> = Vec::default();

    let mut model: Model<asn1rs_model::model::Asn> = Model {
        name: "__proc_macro".to_string(),
        imports: vec![],
        definitions: vec![],
    };

    let item = match item {
        Item::Struct(mut strct) if asn_type_decl == "sequence" => {
            let mut fields = Vec::new();
            for field in strct.fields.iter_mut() {
                let mut removed = None;
                'inner: for i in 0..field.attrs.len() {
                    if field.attrs[i]
                        .path
                        .segments
                        .first()
                        .unwrap()
                        .ident
                        .to_string()
                        .eq("asn")
                    {
                        removed = Some(field.attrs.remove(i));
                        break 'inner;
                    }
                }
                if let Some(removed) = removed {
                    match removed.parse_args::<Asn>() {
                        Ok(asn) => {
                            fields.push(Field {
                                name: field.ident.as_ref().map(ToString::to_string).unwrap(),
                                optional: asn.optional,
                                role: match into_asn(&field.ty, asn) {
                                    Some(asn) => asn,
                                    None => {
                                        return TokenStream::from(
                                            syn::Error::new(field.span(), "Missing ASN-Type")
                                                .to_compile_error(),
                                        );
                                    }
                                },
                            });
                        }
                        Err(e) => return TokenStream::from(e.to_compile_error()),
                    }
                }
            }
            println!("---------- parsed");
            let definition = Definition(strct.ident.to_string(), Type::Sequence(fields).untagged());
            println!("{:#?}", definition);
            model.definitions.push(definition);

            println!("---------- output");
            let st = Item::Struct(strct.clone());
            println!("{}", TokenStream::from(quote! {#st}).to_string());

            Item::Struct(strct)
        }
        Item::Enum(enm) if asn_type_decl == "enumerated" => {
            let plain_enum = enm.variants.iter().all(|v| v.fields.is_empty());
            let variants = enm
                .variants
                .iter()
                .map(|v| v.ident.to_string())
                .collect::<Vec<_>>();
            if plain_enum {
                // TODO extensible
                // TODO tags
                let enumerated = Enumerated::from_names(variants.into_iter());
                model.definitions.push(Definition(
                    enm.ident.to_string(),
                    Type::Enumerated(enumerated).untagged(),
                ));
            } else {
                // data enum
                panic!("ENUMERATED does not allow data carried on Variants. Consider type CHOICE");
            }

            Item::Enum(enm)
        }
        Item::Enum(mut enm) if asn_type_decl == "choice" => {
            let data_enum = enm.variants.iter().all(|v| !v.fields.is_empty());
            let variants = enm
                .variants
                .iter_mut()
                .map(|v| {
                    if v.fields.len() != 1 {
                        panic!("Variants of CHOICE have to have exactly one field");
                    }
                    let mut attr = None;
                    'inner: for i in 0..v.attrs.len() {
                        if v.attrs[i]
                            .path
                            .segments
                            .first()
                            .unwrap()
                            .ident
                            .to_string()
                            .eq("asn")
                        {
                            attr = Some(v.attrs.remove(i));
                            break 'inner;
                        }
                    }
                    let attr = attr.expect("Missing #[asn(..)] attribute");

                    match attr.parse_args::<Asn>() {
                        Ok(asn) => match into_asn(&v.fields.iter().next().unwrap().ty, asn) {
                            Some(asn) => {
                                let name = v.ident.to_string();
                                Ok(ChoiceVariant {
                                    name,
                                    tag: asn.tag,
                                    r#type: asn.r#type,
                                })
                            }
                            None => Err(TokenStream::from(
                                syn::Error::new(v.span(), "Missing ASN-Type").to_compile_error(),
                            )),
                        },
                        Err(e) => Err(TokenStream::from(e.to_compile_error())),
                    }
                })
                .collect::<Vec<_>>();

            if data_enum {
                // TODO extensible
                // TODO tags
                let choice = Choice::from_variants({
                    let mut new = Vec::with_capacity(variants.len());
                    for var in variants {
                        new.push(match var {
                            Ok(variant) => variant,
                            Err(e) => return e,
                        });
                    }
                    new
                });
                model.definitions.push(Definition(
                    enm.ident.to_string(),
                    Type::Choice(choice).untagged(),
                ));
            } else {
                // mixed case
                panic!("CHOICE does not allow any Variant to not have data attached!");
            }
            Item::Enum(enm)
        }
        item => item,
    };

    if !model.definitions.is_empty() {
        let model_rust = model.to_rust();

        use asn1rs_model::gen::rust::walker::AsnDefWalker;
        let stringified = AsnDefWalker::stringify(&model_rust);
        additional_impl.push(TokenStream2::from_str(&stringified).unwrap());
    }

    let result = TokenStream::from(quote! {
        #item
        #(#additional_impl)*
    });

    println!("---------- result");
    println!("{}", result.to_string());
    result
}

fn into_asn(ty: &syn::Type, asn: Asn) -> Option<asn1rs_model::model::Asn> {
    Some(asn1rs_model::model::Asn {
        tag: asn.tag,
        r#type: match asn.r#type {
            Some(some) => {
                if let Type::TypeReference(_) = some {
                    Type::TypeReference(quote! { #ty }.to_string())
                } else {
                    some
                }
            }
            None => return None,
        },
    })
}

#[derive(Debug, Default)]
struct Asn {
    r#type: Option<Type>,
    tag: Option<Tag>,
    // TODO allow nested optional
    /// this needs refactoring as well as [`Type`] because it does not support nested `Optional`s
    ///
    /// [`Type`]: asn1rs_model::model::Type  
    optional: bool,
}

impl Parse for Asn {
    fn parse<'a>(input: &'a ParseBuffer<'a>) -> syn::Result<Self> {
        let mut asn = Self::default();
        let mut first = true;

        while !input.cursor().eof() {
            let ident = input.step(|c| c.ident().ok_or_else(|| c.error("Expected ASN-Type")))?;
            match ident.to_string().to_lowercase().as_str() {
                "utf8string" if first => asn.r#type = Some(Type::UTF8String),
                "octet_string" if first => asn.r#type = Some(Type::OctetString),
                "integer" if first => {
                    let range = MaybeRanged::parse(input)?;
                    asn.r#type = Some(Type::Integer(range.0.map(|(min, max)| Range(min, max))));
                }
                "complex" if first => asn.r#type = Some(Type::TypeReference(String::default())),
                "tag" if !first => {
                    let tag = AttrTag::parse(input)?;
                    asn.tag = Some(tag.0);
                }
                // TODO allow nested optional, see comment in Asn above
                "optional" if !first => {
                    asn.optional = true;
                }
                r#type => return Err(input.error(format!("Unexpected attribute: `{}`", r#type))),
            }
            if !input.cursor().eof() && !input.peek(syn::token::Comma) {
                return Err(input.error("Attributes must be separated by comma"));
            } else if !input.cursor().eof() {
                let _ = input.step(|c| {
                    c.punct()
                        .ok_or_else(|| input.error("Attributes must be separated by comma"))
                })?;
            }
            first = false;
        }
        Ok(asn)
    }
}
