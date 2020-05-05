mod attribute;
mod range;
mod tag;

use asn1rs_model::model::Asn as AsnModelType;
use asn1rs_model::model::{Choice, ChoiceVariant, Definition, Enumerated, Field, Model, Type};
use attribute::AsnAttribute;
use proc_macro::TokenStream;
use quote::quote;
use std::convert::Infallible;
use std::str::FromStr;
use syn::export::TokenStream2;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, AttributeArgs, Meta};
use syn::{Item, NestedMeta};

pub(crate) fn parse(attr: TokenStream, item: TokenStream) -> TokenStream {
    if cfg!(feature = "debug-proc-macro") {
        println!();
        println!("---------- asn proc_macro_attribute parse call ----------");
        println!("Attribute: {}", attr.to_string());
        println!("Item:      {}", item.to_string());
        println!();
    }

    // a bit sad to do a clone just to be able to retrieve the span
    let attr_span = TokenStream2::from(attr.clone()).span();

    let attributes = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    let asn_type_decl = match attributes.get(0) {
        None => return compile_error_ts(attr_span, "Missing ASN attribute"),
        Some(NestedMeta::Meta(Meta::Path(path))) => path
            .segments
            .iter()
            .next()
            .expect("Missing ASN Attribute in path")
            .ident
            .to_string()
            .to_lowercase(),
        _ => return compile_error_ts(attr_span, "Invalid ASN Attribute type"),
    };

    let mut additional_impl: Vec<TokenStream2> = Vec::default();

    let mut model: Model<AsnModelType> = Model {
        name: "__proc_macro".to_string(),
        imports: vec![],
        definitions: vec![],
    };

    let (definition, item) = match parse_item_definition(item, &asn_type_decl) {
        Ok(result) => result,
        Err(e) => return e,
    };

    if cfg!(feature = "debug-proc-macro") {
        println!("---------- parsed definition begin ----------");
        println!("{:#?}", definition);
        println!("---------- parsed definition end ----------");
        println!();

        println!("---------- filtered item begin ----------");
        println!("{}", TokenStream::from(quote! {#item}).to_string());
        println!("---------- filtered item end ----------");
        println!();
    }

    if let Some(definition) = definition {
        model.definitions.push(definition);
        use asn1rs_model::gen::rust::walker::AsnDefWriter;
        additional_impl
            .push(TokenStream2::from_str(&AsnDefWriter::stringify(&model.to_rust())).unwrap());
    }

    let result = TokenStream::from(quote! {
        #item
        #(#additional_impl)*
    });

    if cfg!(feature = "debug-proc-macro") {
        println!("---------- result begin ----------");
        println!("{}", result.to_string());
        println!("---------- result end ----------");
        println!();
    }
    result
}

fn parse_item_definition(
    item: syn::Item,
    asn_type_decl: &str,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    match item {
        Item::Struct(strct) if asn_type_decl == "sequence" => parse_sequence(strct),
        Item::Struct(strct) if asn_type_decl == "transparent" => parse_transparent(strct),
        Item::Enum(enm) if asn_type_decl == "enumerated" => parse_enumerated(enm),
        Item::Enum(enm) if asn_type_decl == "choice" => parse_choice(enm),
        item => Ok((None, item)),
    }
}

fn parse_sequence(
    mut strct: syn::ItemStruct,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    let fields = strct
        .fields
        .iter_mut()
        .map(|field| {
            if field.ident.is_none() {
                compile_err_ts(
                    field.span(),
                    "Unnamed fields are not allowed here. Consider transparent type",
                )?;
            }

            parse_and_remove_first_asn_attribute(field.span(), &field.ty, &mut field.attrs).map(
                |asn| Field {
                    name: field.ident.as_ref().unwrap().to_string(),
                    role: asn,
                },
            )
        })
        .vec_result()?;

    Ok((
        Some(Definition(
            strct.ident.to_string(),
            Type::Sequence(fields).untagged(),
        )),
        Item::Struct(strct),
    ))
}

fn parse_transparent(
    mut strct: syn::ItemStruct,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    if strct.fields.len() != 1 || strct.fields.iter().next().unwrap().ident.is_some() {
        compile_err_ts(
            strct.span(),
            "Transparent structs have to have exactly one unnamed field",
        )?;
    }

    let field = strct.fields.iter_mut().next().unwrap();
    parse_and_remove_first_asn_attribute(field.span(), &field.ty, &mut field.attrs).map(|asn| {
        (
            Some(Definition(strct.ident.to_string(), asn)),
            Item::Struct(strct),
        )
    })
}

fn parse_enumerated(
    enm: syn::ItemEnum,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    enm.variants
        .iter()
        .find(|v| !v.fields.is_empty())
        .map(|v| {
            compile_err_ts(
                v.span(),
                "ENUMERATED does not allow data carried on Variants. Consider type CHOICE",
            )
        })
        .transpose()?;

    // TODO extensible
    // TODO tags
    let enumerated = Enumerated::from_names(enm.variants.iter().map(|v| v.ident.to_string()));

    Ok((
        Some(Definition(
            enm.ident.to_string(),
            Type::Enumerated(enumerated).untagged(),
        )),
        Item::Enum(enm),
    ))
}

fn parse_choice(
    mut enm: syn::ItemEnum,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    enm.variants
        .iter()
        .find(|v| v.fields.is_empty())
        .map(|v| {
            compile_err_ts(
                v.span(),
                "CHOICE does not allow any variant to not have data attached",
            )
        })
        .transpose()?;

    let variants = enm
        .variants
        .iter_mut()
        .map(|v| {
            if v.fields.len() != 1 || v.fields.iter().next().unwrap().ident.is_some() {
                compile_err_ts(
                    v.span(),
                    "Variants of CHOICE have to have exactly one unnamed field",
                )?;
            }

            parse_and_remove_first_asn_attribute(
                v.span(),
                &v.fields.iter().next().unwrap().ty,
                &mut v.attrs,
            )
            .map(|asn| {
                // TODO extensible
                // TODO tags
                ChoiceVariant {
                    name: v.ident.to_string(),
                    tag: asn.tag,
                    r#type: asn.r#type,
                }
            })
        })
        .vec_result()?;

    Ok((
        Some(Definition(
            enm.ident.to_string(),
            Type::Choice(Choice::from_variants(variants)).untagged(),
        )),
        Item::Enum(enm),
    ))
}

fn parse_and_remove_first_asn_attribute(
    span: proc_macro2::Span,
    ty: &syn::Type,
    attrs: &mut Vec<Attribute>,
) -> Result<AsnModelType, TokenStream> {
    find_and_remove_first_asn_attribute_or_err(span, attrs)
        .and_then(|attribute| {
            attribute
                .parse_args::<AsnAttribute>()
                .map_err(|e| TokenStream::from(e.to_compile_error()))
        })
        .and_then(|asn| into_asn_or_err(span, &ty, asn))
}

fn into_asn_or_err(
    span: proc_macro2::Span,
    ty: &syn::Type,
    asn: AsnAttribute,
) -> Result<AsnModelType, TokenStream> {
    into_asn(ty, asn).ok_or_else(|| compile_error_ts(span, "Missing ASN-Type"))
}

fn into_asn(ty: &syn::Type, asn: AsnAttribute) -> Option<AsnModelType> {
    Some(AsnModelType {
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

fn compile_err_ts<T: std::fmt::Display>(
    span: proc_macro2::Span,
    msg: T,
) -> Result<Infallible, TokenStream> {
    Err(compile_error_ts(span, msg))
}

fn compile_error_ts<T: std::fmt::Display>(span: proc_macro2::Span, msg: T) -> TokenStream {
    TokenStream::from(compile_error_ts2(span, msg))
}

fn compile_error_ts2<T: std::fmt::Display>(span: proc_macro2::Span, msg: T) -> TokenStream2 {
    syn::Error::new(span, msg).to_compile_error()
}

fn find_and_remove_first_asn_attribute_or_err(
    span: proc_macro2::Span,
    attributes: &mut Vec<Attribute>,
) -> Result<Attribute, TokenStream> {
    find_and_remove_first_asn_attribute(attributes)
        .ok_or_else(|| compile_error_ts(span, "Missing #[asn(...)] attribute"))
}

fn find_and_remove_first_asn_attribute(attributes: &mut Vec<Attribute>) -> Option<Attribute> {
    for i in 0..attributes.len() {
        if attributes[i]
            .path
            .segments
            .first()
            .unwrap()
            .ident
            .to_string()
            .eq("asn")
        {
            return Some(attributes.remove(i));
        }
    }
    None
}

trait VecResult<T, E> {
    fn vec_result(self) -> Result<Vec<T>, E>
    where
        Self: Sized;
}

impl<T, E, I: ExactSizeIterator<Item = Result<T, E>>> VecResult<T, E> for I {
    fn vec_result(self) -> Result<Vec<T>, E>
    where
        Self: Sized,
    {
        let mut result = Vec::with_capacity(self.len());
        for value in self {
            result.push(value?);
        }
        Ok(result)
    }
}
