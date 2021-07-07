use crate::model::charset::Charset;
use crate::model::{Asn, Definition, Model, Tag, TagProperty, Type};

pub struct TagResolver<'a> {
    model: &'a Model<Asn>,
    scope: &'a [&'a Model<Asn>],
}

impl TagResolver<'_> {
    pub const fn new<'a>(model: &'a Model<Asn>, scope: &'a [&'a Model<Asn>]) -> TagResolver<'a> {
        TagResolver { model, scope }
    }

    pub fn resolve_default(ty: &Type) -> Option<Tag> {
        let model = Model::<Asn>::default();
        TagResolver {
            model: &model,
            scope: &[],
        }
        .resolve_type_tag(ty)
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_tag(&self, ty: &str) -> Option<Tag> {
        self.model
            .imports
            .iter()
            .find(|import| import.what.iter().any(|what| what.eq(ty)))
            .map(|import| &import.from)
            .and_then(|model_name| self.scope.iter().find(|model| model.name.eq(model_name)))
            .and_then(|model| {
                TagResolver {
                    model,
                    scope: self.scope,
                }
                .resolve_tag(ty)
            })
            .or_else(|| {
                self.model.definitions.iter().find(|d| d.0.eq(ty)).and_then(
                    |Definition(_name, asn)| asn.tag.or_else(|| self.resolve_type_tag(&asn.r#type)),
                )
            })
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_no_default(&self, ty: &Type) -> Option<Tag> {
        let default = Self::resolve_default(ty);
        let resolved = self.resolve_type_tag(ty);
        resolved.filter(|r| default.ne(&Some(*r)))
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    /// ITU-T X.680 | ISO/IEC 8824-1, 41, table 8
    pub fn resolve_type_tag(&self, ty: &Type) -> Option<Tag> {
        match ty {
            Type::Boolean => Some(Tag::DEFAULT_BOOLEAN),
            Type::Integer(_) => Some(Tag::DEFAULT_INTEGER),
            Type::BitString(_) => Some(Tag::DEFAULT_BIT_STRING),
            Type::OctetString(_) => Some(Tag::DEFAULT_OCTET_STRING),
            Type::Enumerated(_) => Some(Tag::DEFAULT_ENUMERATED),
            Type::String(_, Charset::Numeric) => Some(Tag::DEFAULT_NUMERIC_STRING),
            Type::String(_, Charset::Printable) => Some(Tag::DEFAULT_PRINTABLE_STRING),
            Type::String(_, Charset::Visible) => Some(Tag::DEFAULT_VISIBLE_STRING),
            Type::String(_, Charset::Utf8) => Some(Tag::DEFAULT_UTF8_STRING),
            Type::String(_, Charset::Ia5) => Some(Tag::DEFAULT_IA5_STRING),
            Type::Null => Some(Tag::DEFAULT_NULL),
            Type::Optional(inner) => self.resolve_type_tag(&**inner),
            Type::Default(inner, ..) => self.resolve_type_tag(&**inner),
            Type::Sequence(_) => Some(Tag::DEFAULT_SEQUENCE),
            Type::SequenceOf(_, _) => Some(Tag::DEFAULT_SEQUENCE_OF),
            Type::Set(_) => Some(Tag::DEFAULT_SET),
            Type::SetOf(_, _) => Some(Tag::DEFAULT_SET_OF),
            Type::Choice(choice) => {
                let mut tags = choice
                    .variants()
                    .take(
                        choice
                            .extension_after_index()
                            .map(|extension_after| extension_after + 1)
                            .unwrap_or_else(|| choice.len()),
                    )
                    .map(|v| v.tag().or_else(|| self.resolve_type_tag(v.r#type())))
                    .collect::<Option<Vec<Tag>>>()?;
                tags.sort();
                if cfg!(feature = "debug-proc-macro") {
                    println!("resolved::::{:?}", tags);
                }
                tags.into_iter().next()
            }
            Type::TypeReference(inner, tag) => {
                let tag = (*tag).or_else(|| self.resolve_tag(inner.as_str()));
                if cfg!(feature = "debug-proc-macro") {
                    println!("resolved :: {}::Tag = {:?}", inner, tag);
                }
                tag
            }
        }
    }
}
