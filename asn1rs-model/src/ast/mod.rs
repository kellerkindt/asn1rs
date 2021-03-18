mod attribute;
mod constants;
mod range;
mod size;
mod tag;

use crate::ast::attribute::{Context, DefinitionHeader, Transparent};
use crate::ast::constants::ConstLit;
use crate::model::lor::Resolved;
use crate::model::{Choice, ChoiceVariant, Definition, Enumerated, Field, Model, Type};
use crate::model::{ComponentTypeList, EnumeratedVariant, TagProperty, TagResolver};
use attribute::AsnAttribute;
use proc_macro2::TokenStream;
use quote::quote;
use std::convert::Infallible;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{Attribute, Item};

type AsnModelType = crate::model::Asn<crate::model::lor::Resolved>;

pub fn parse(attr: TokenStream, item: TokenStream) -> TokenStream {
    if cfg!(feature = "debug-proc-macro") {
        println!();
        println!("---------- asn proc_macro_attribute parse call ----------");
        println!("Attribute: {}", attr.to_string());
        println!("Item:      {}", item.to_string());
        println!();
    }

    let (definition, item) = match parse_asn_definition(attr, item) {
        Ok(v) => v,
        Err(e) => return e,
    };

    if cfg!(feature = "debug-proc-macro") {
        println!("---------- parsed definition begin ----------");
        println!("{:#?}", definition);
        println!("---------- parsed definition end ----------");
        println!();

        println!("---------- filtered item begin ----------");
        println!("{}", quote! {#item}.to_string());
        println!("---------- filtered item end ----------");
        println!();
    }

    let additional_impl = expand(definition);

    let result = quote! {
        #item
        #(#additional_impl)*
    };

    if cfg!(feature = "debug-proc-macro") {
        println!("---------- result begin ----------");
        println!("{}", result.to_string());
        println!("---------- result end ----------");
        println!();
    }
    result
}

pub fn expand(definition: Option<Definition<AsnModelType>>) -> Vec<TokenStream> {
    let mut additional_impl: Vec<TokenStream> = Vec::default();
    let mut model: Model<AsnModelType> = Model {
        name: "__proc_macro".to_string(),
        ..Default::default()
    };

    if let Some(definition) = definition {
        model.definitions.push(definition);
        use crate::gen::rust::walker::AsnDefWriter;

        if cfg!(feature = "debug-proc-macro") {
            println!("---------- parsed definition to rust begin ----------");
            println!("{:?}", model.to_rust());
            println!("---------- parsed definition to rust end ----------");
            println!();
        }
        additional_impl
            .push(TokenStream::from_str(&AsnDefWriter::stringify(&model.to_rust())).unwrap());
    }

    additional_impl
}

pub fn parse_asn_definition(
    attr: TokenStream,
    item: TokenStream,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    let item_span = item.span();
    let attr_span = attr.span();

    if cfg!(feature = "debug-proc-macro") {
        println!("ATTRIBUTE: {}", attr.to_string());
        println!("ITEM:      {}", item.to_string());
    }

    let item = syn::parse2::<Item>(item)
        .map_err(|e| compile_error_ts(item_span, format!("Invalid Item: {}", e)))?;
    let asn = syn::parse2::<AsnAttribute<DefinitionHeader>>(attr.clone()).map_err(|e| {
        compile_error_ts(
            attr_span,
            format!("Invalid ASN attribute ('{}'): {}", attr.to_string(), e),
        )
    })?;

    match item {
        Item::Struct(strct) if asn.primary.eq_ignore_ascii_case("sequence") => {
            parse_sequence_or_set(strct, &asn, attr_span, Type::Sequence)
        }
        Item::Struct(strct) if asn.primary.eq_ignore_ascii_case("set") => {
            parse_sequence_or_set(strct, &asn, attr_span, Type::Set)
        }
        Item::Struct(strct) if asn.primary.eq_ignore_ascii_case("transparent") => {
            parse_transparent(strct, &asn, attr_span)
        }
        Item::Enum(enm) if asn.primary.eq_ignore_ascii_case("enumerated") => {
            parse_enumerated(enm, &asn, attr_span)
        }
        Item::Enum(enm) if asn.primary.eq_ignore_ascii_case("choice") => {
            parse_choice(enm, &asn, attr_span)
        }
        item => Ok((None, item)),
    }
}

fn parse_sequence_or_set<F: Fn(ComponentTypeList<Resolved>) -> Type>(
    mut strct: syn::ItemStruct,
    asn: &AsnAttribute<DefinitionHeader>,
    asn_span: proc_macro2::Span,
    mapper: F,
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

            parse_and_remove_first_asn_attribute_type::<Transparent>(
                field.span(),
                &field.ty,
                &mut field.attrs,
            )
            .map(|asn| Field {
                name: field.ident.as_ref().unwrap().to_string(),
                role: asn,
            })
        })
        .vec_result()?;

    Ok((
        Some(Definition(
            strct.ident.to_string(),
            mapper(ComponentTypeList {
                extension_after: find_extensible_index(
                    asn,
                    asn_span,
                    fields.iter().map(|v| &v.name),
                )?,
                fields,
            })
            .opt_tagged(asn.tag),
        )),
        Item::Struct(strct),
    ))
}

fn parse_transparent(
    mut strct: syn::ItemStruct,
    asn: &AsnAttribute<DefinitionHeader>,
    _asn_span: proc_macro2::Span,
) -> Result<(Option<Definition<AsnModelType>>, Item), TokenStream> {
    if strct.fields.len() != 1 || strct.fields.iter().next().unwrap().ident.is_some() {
        compile_err_ts(
            strct.span(),
            "Transparent structs have to have exactly one unnamed field",
        )?;
    }

    let field = strct.fields.iter_mut().next().unwrap();
    parse_and_remove_first_asn_attribute_type::<Transparent>(
        field.span(),
        &field.ty,
        &mut field.attrs,
    )
    .map(|parsed| {
        (
            Some(Definition(
                strct.ident.to_string(),
                parsed.with_tag_opt(asn.tag),
            )),
            Item::Struct(strct),
        )
    })
}

fn parse_enumerated(
    mut enm: syn::ItemEnum,
    asn: &AsnAttribute<DefinitionHeader>,
    asn_span: proc_macro2::Span,
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

    let variants = enm
        .variants
        .iter_mut()
        .map(|v| {
            let variant = EnumeratedVariant::from_name(v.ident.to_string());
            let attributes = index_of_first_asn_attribute(&v.attrs).map(|_index| {
                parse_and_remove_first_asn_attribute::<EnumeratedVariant>(v.span(), &mut v.attrs)
            });
            if let Some(attributes) = attributes {
                attributes.and_then(|attr| {
                    if attr.tag.is_some() {
                        compile_err_ts(v.span(), "ENUMERATED Variants must not have a Tag")?;
                    }

                    Ok(variant.with_number_opt(attr.primary))
                })
            } else {
                Ok(variant)
            }
        })
        .vec_result()?;

    let extension_after = find_extensible_index(asn, asn_span, variants.iter().map(|v| v.name()))?;
    let enumerated =
        Enumerated::from_variants(variants).with_maybe_extension_after(extension_after);

    Ok((
        Some(Definition(
            enm.ident.to_string(),
            Type::Enumerated(enumerated).opt_tagged(asn.tag),
        )),
        Item::Enum(enm),
    ))
}

fn parse_choice(
    mut enm: syn::ItemEnum,
    asn: &AsnAttribute<DefinitionHeader>,
    asn_span: proc_macro2::Span,
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

            parse_and_remove_first_asn_attribute_type::<ChoiceVariant>(
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

    let extensible_after =
        find_extensible_index(&asn, asn_span, variants.iter().map(|v| v.name()))?;

    let choice = Type::Choice(
        Choice::from_variants(variants.into_iter()).with_maybe_extension_after(extensible_after),
    );

    let tag = asn.tag.or_else(|| TagResolver::resolve_default(&choice));

    Ok((
        Some(Definition(enm.ident.to_string(), choice.opt_tagged(tag))),
        Item::Enum(enm),
    ))
}

fn find_extensible_index(
    asn: &AsnAttribute<DefinitionHeader>,
    asn_span: proc_macro2::Span,
    variants: impl Iterator<Item = impl AsRef<str>>,
) -> Result<Option<usize>, TokenStream> {
    asn.extensible_after
        .as_ref()
        .map(|name| {
            variants
                .enumerate()
                .find_map(|(index, v)| {
                    if v.as_ref().eq(name) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    compile_error_ts(asn_span, "Cannot find variant for extensible attribute")
                })
        })
        .transpose()
}

fn parse_and_remove_first_asn_attribute_type<C: Context<Primary = Type>>(
    span: proc_macro2::Span,
    ty: &syn::Type,
    attrs: &mut Vec<Attribute>,
) -> Result<AsnModelType, TokenStream> {
    parse_and_remove_first_asn_attribute::<C>(span, attrs).map(|asn| into_asn(&ty, asn))
}

fn parse_and_remove_first_asn_attribute<C: Context>(
    span: proc_macro2::Span,
    attrs: &mut Vec<Attribute>,
) -> Result<AsnAttribute<C>, TokenStream> {
    find_and_remove_first_asn_attribute_or_err(span, attrs).and_then(|attribute| {
        attribute
            .parse_args::<AsnAttribute<C>>()
            .map_err(|e| e.to_compile_error())
    })
}

fn into_asn<C: Context<Primary = Type>>(ty: &syn::Type, mut asn: AsnAttribute<C>) -> AsnModelType {
    AsnModelType {
        tag: asn.tag,
        r#type: if let Type::TypeReference(_, empty_tag) = asn.primary {
            Type::TypeReference(quote! { #ty }.to_string(), empty_tag.or(asn.tag))
        } else {
            if let Type::Integer(int) = asn.primary.no_optional_mut() {
                asn.consts
                    .into_iter()
                    .map(|c| match c {
                        ConstLit::I64(name, value) => (name, value),
                    })
                    .for_each(|v| int.constants.push(v));
            }
            asn.primary
        },
    }
}

fn compile_err_ts<T: std::fmt::Display>(
    span: proc_macro2::Span,
    msg: T,
) -> Result<Infallible, TokenStream> {
    Err(compile_error_ts(span, msg))
}

fn compile_error_ts<T: std::fmt::Display>(span: proc_macro2::Span, msg: T) -> TokenStream {
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
    index_of_first_asn_attribute(&attributes[..]).map(|index| attributes.remove(index))
}

fn index_of_first_asn_attribute(attributes: &[Attribute]) -> Option<usize> {
    attributes.iter().enumerate().find_map(|(index, attr)| {
        attr.path
            .segments
            .first()
            .filter(|s| s.ident.to_string().eq("asn"))
            .map(|_| index)
    })
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
