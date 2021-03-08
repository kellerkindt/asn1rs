use crate::model::rust::Field as RustField;
use crate::model::ComponentTypeList;
use crate::model::Context;
use crate::model::{Asn, ChoiceVariant};
use crate::model::{Charset, Range};
use crate::model::{Definition, Type};
use crate::model::{Import, Tag, TagProperty};
use crate::model::{Model, Size};
use crate::model::{TagResolver, Type as AsnType};

const I8_MAX: i64 = i8::max_value() as i64;
const I16_MAX: i64 = i16::max_value() as i64;
const I32_MAX: i64 = i32::max_value() as i64;
//const I64_MAX: i64 = i64::max_value() as i64;

const U8_MAX: u64 = u8::max_value() as u64;
const U16_MAX: u64 = u16::max_value() as u64;
const U32_MAX: u64 = u32::max_value() as u64;
//const U64_MAX: u64 = u64::max_value() as u64;

pub type PlainVariant = String;
pub type PlainEnum = Enumeration<PlainVariant>;
pub type DataEnum = Enumeration<DataVariant>;

/// Integers are ordered where Ixx < Uxx so
/// that when comparing two instances `RustType`
/// and a > b, then the integer type of a can
/// use ::from(..) to cast from b
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RustType {
    Bool,
    I8(Range<i8>),
    U8(Range<u8>),
    I16(Range<i16>),
    U16(Range<u16>),
    I32(Range<i32>),
    U32(Range<u32>),
    I64(Range<i64>),
    U64(Range<Option<u64>>),
    String(Size, Charset),
    VecU8(Size),
    BitVec(Size),
    Vec(Box<RustType>, Size, EncodingOrdering),
    Option(Box<RustType>),

    /// Indicates a complex, custom type that is
    /// not one of rusts known types. This can be
    /// thought of as a "ReferenceType"; declaring usage,
    /// but not being declared here
    Complex(String, Option<Tag>),
}

impl RustType {
    pub fn as_inner_type(&self) -> &RustType {
        if self.is_primitive() {
            return self;
        }
        if let RustType::Vec(inner, ..) | RustType::Option(inner) = self {
            inner.as_inner_type()
        } else {
            self
        }
    }

    pub fn into_inner_type(self) -> RustType {
        if self.is_primitive() {
            return self;
        }
        if let RustType::Vec(inner, ..) | RustType::Option(inner) = self {
            inner.into_inner_type()
        } else {
            self
        }
    }

    pub fn to_inner(&self) -> Option<String> {
        if self.is_primitive() {
            return Some(self.to_string());
        }
        if let RustType::Vec(inner, ..) | RustType::Option(inner) = self {
            inner.to_inner()
        } else {
            None
        }
    }

    pub fn to_inner_type_string(&self) -> String {
        self.to_inner().unwrap_or_else(|| self.to_string())
    }

    pub fn no_option(self) -> Self {
        match self {
            RustType::Option(inner) => *inner,
            rust => rust,
        }
    }

    pub fn as_no_option(&self) -> &Self {
        if let RustType::Option(inner) = self {
            inner.as_no_option()
        } else {
            self
        }
    }

    pub fn is_vec(&self) -> bool {
        matches!(self.as_no_option(), RustType::Vec(..))
    }

    pub fn is_option(&self) -> bool {
        matches!(self, RustType::Option(..))
    }

    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            RustType::Bool
                | RustType::U8(_)
                | RustType::I8(_)
                | RustType::U16(_)
                | RustType::I16(_)
                | RustType::U32(_)
                | RustType::I32(_)
                | RustType::U64(_)
                | RustType::I64(_)
        )
    }

    pub fn integer_range_str(&self) -> Option<Range<String>> {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            RustType::Bool => None,
            RustType::U8(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::I8(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::U16(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::I16(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::U32(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::I32(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::U64(Range(min, max, extensible)) => Some(Range(
                min.unwrap_or_default().to_string(),
                max.unwrap_or_else(|| i64::max_value() as u64).to_string(),
                *extensible,
            )),
            RustType::I64(Range(min, max, extensible)) => {
                Some(Range(min.to_string(), max.to_string(), *extensible))
            }
            RustType::String(..) => None,
            RustType::VecU8(_) => None,
            RustType::BitVec(_) => None,
            RustType::Vec(inner, _size, _ordering) => inner.integer_range_str(),
            RustType::Option(inner) => inner.integer_range_str(),
            RustType::Complex(_, _) => None,
        }
    }

    pub fn into_asn(self) -> AsnType {
        match self {
            RustType::Bool => AsnType::Boolean,
            RustType::I8(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::U8(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::I16(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::U16(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::I32(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::U32(Range(min, max, extensible)) => AsnType::integer_with_range(Range(
                Some(i64::from(min)),
                Some(i64::from(max)),
                extensible,
            )),
            RustType::I64(Range(min, max, extensible)) => {
                AsnType::integer_with_range(Range(Some(min), Some(max), extensible))
            }
            RustType::U64(range) => AsnType::integer_with_range(Range(
                range.min().map(|v| v as i64),
                range.max().map(|v| v as i64),
                range.extensible(),
            )),
            RustType::String(size, charset) => AsnType::String(size, charset),
            RustType::VecU8(size) => AsnType::OctetString(size),
            RustType::BitVec(size) => AsnType::bit_vec_with_size(size),
            RustType::Vec(inner, size, EncodingOrdering::Keep) => {
                AsnType::SequenceOf(Box::new(inner.into_asn()), size)
            }
            RustType::Vec(inner, size, EncodingOrdering::Sort) => {
                AsnType::SetOf(Box::new(inner.into_asn()), size)
            }
            RustType::Option(value) => AsnType::Optional(Box::new(value.into_asn())),
            RustType::Complex(name, tag) => AsnType::TypeReference(name, tag),
        }
    }

    pub fn similar(&self, other: &Self) -> bool {
        match self {
            RustType::Bool => RustType::Bool == *other,
            RustType::U8(_) => matches!(other, RustType::U8(_)),
            RustType::I8(_) => matches!(other, RustType::I8(_)),
            RustType::U16(_) => matches!(other, RustType::U16(_)),
            RustType::I16(_) => matches!(other, RustType::I16(_)),
            RustType::U32(_) => matches!(other, RustType::U32(_)),
            RustType::I32(_) => matches!(other, RustType::I32(_)),
            RustType::U64(_) => matches!(other, RustType::U64(_)),
            RustType::I64(_) => matches!(other, RustType::I64(_)),
            RustType::String(..) => matches!(other, RustType::String(..)),
            RustType::VecU8(_) => matches!(other, RustType::VecU8(_)),
            RustType::BitVec(_) => matches!(other, RustType::BitVec(_)),
            RustType::Vec(inner_a, _size, _ordering) => {
                if let RustType::Vec(inner_b, _other_size, _ordering) = other {
                    inner_a.similar(inner_b)
                } else {
                    false
                }
            }
            RustType::Option(inner_a) => {
                if let RustType::Option(inner_b) = other {
                    inner_a.similar(inner_b)
                } else {
                    false
                }
            }
            RustType::Complex(inner_a, _tag) => {
                if let RustType::Complex(inner_b, _tag) = other {
                    inner_a.eq(inner_b)
                } else {
                    false
                }
            }
        }
    }

    /// ITU-T X.680 | ISO/IEC 8824-1, 8.6
    pub fn tag(&self) -> Option<Tag> {
        Some(match self {
            RustType::Bool => Tag::DEFAULT_BOOLEAN,
            RustType::I8(_)
            | RustType::U8(_)
            | RustType::I16(_)
            | RustType::U16(_)
            | RustType::I32(_)
            | RustType::U32(_)
            | RustType::I64(_)
            | RustType::U64(_) => Tag::DEFAULT_INTEGER,
            RustType::BitVec(_) => Tag::DEFAULT_BIT_STRING,
            RustType::VecU8(_) => Tag::DEFAULT_OCTET_STRING,
            RustType::String(_, charset) => charset.default_tag(),
            RustType::Vec(_, _, EncodingOrdering::Keep) => Tag::DEFAULT_SEQUENCE_OF,
            RustType::Vec(_, _, EncodingOrdering::Sort) => Tag::DEFAULT_SET_OF,
            RustType::Option(inner) => return inner.tag(),
            // TODO this is wrong. This should resolve the tag from the referenced type instead, but atm the infrastructure is missing to do such a thing, see github#13
            RustType::Complex(_, tag) => return *tag,
        })
    }
}

/// Describes whether the original declaration cares about (re-)ordering the elements or whether
/// their encoding is to be applied in the order of definition (struct fields) or appearance (vec)
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
pub enum EncodingOrdering {
    Sort,
    Keep,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Rust {
    Struct {
        ordering: EncodingOrdering,
        fields: Vec<Field>,
        tag: Option<Tag>,
        extension_after: Option<usize>,
    },
    Enum(PlainEnum),
    DataEnum(DataEnum),

    /// Used to represent a single, unnamed inner type
    TupleStruct {
        r#type: RustType,
        tag: Option<Tag>,
        constants: Vec<(String, String)>,
    },
}

impl Rust {
    #[cfg(test)]
    pub fn struct_from_fields(fields: Vec<Field>) -> Self {
        Self::Struct {
            ordering: EncodingOrdering::Keep,
            fields,
            tag: None,
            extension_after: None,
        }
    }

    pub fn tuple_struct_from_type(r#type: RustType) -> Self {
        Self::TupleStruct {
            r#type,
            tag: None,
            constants: Vec::default(),
        }
    }
}

impl TagProperty for Rust {
    fn tag(&self) -> Option<Tag> {
        match self {
            Rust::Struct { tag, .. } => *tag,
            Rust::Enum(e) => e.tag(),
            Rust::DataEnum(c) => c.tag(),
            Rust::TupleStruct { tag, .. } => *tag,
        }
    }

    fn set_tag(&mut self, new_tag: Tag) {
        match self {
            Rust::Struct { tag, .. } => *tag = Some(new_tag),
            Rust::Enum(e) => e.set_tag(new_tag),
            Rust::DataEnum(c) => c.set_tag(new_tag),
            Rust::TupleStruct { tag, .. } => *tag = Some(new_tag),
        }
    }

    fn reset_tag(&mut self) {
        match self {
            Rust::Struct { tag, .. } => *tag = None,
            Rust::Enum(e) => e.reset_tag(),
            Rust::DataEnum(c) => c.reset_tag(),
            Rust::TupleStruct { tag, .. } => *tag = None,
        }
    }
}

impl ToString for RustType {
    fn to_string(&self) -> String {
        match self {
            RustType::Bool => "bool",
            RustType::U8(_) => "u8",
            RustType::I8(_) => "i8",
            RustType::U16(_) => "u16",
            RustType::I16(_) => "i16",
            RustType::U32(_) => "u32",
            RustType::I32(_) => "i32",
            RustType::U64(_) => "u64",
            RustType::I64(_) => "i64",
            RustType::String(..) => "String",
            RustType::VecU8(_) => "Vec<u8>",
            RustType::BitVec(_) => "BitVec",
            RustType::Vec(inner, _size, _ordering) => return format!("Vec<{}>", inner.to_string()),
            RustType::Option(inner) => return format!("Option<{}>", inner.to_string()),
            RustType::Complex(name, _) => return name.clone(),
        }
        .into()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Field {
    pub(crate) name_type: (String, RustType),
    pub(crate) tag: Option<Tag>,
    pub(crate) constants: Vec<(String, String)>,
}

impl Field {
    pub fn from_name_type<T: ToString>(name: T, r#type: RustType) -> Self {
        Self {
            name_type: (name.to_string(), r#type),
            tag: None,
            constants: Vec::default(),
        }
    }

    pub fn fallback_representation(&self) -> &(String, RustType) {
        &self.name_type
    }

    pub fn name(&self) -> &str {
        &self.name_type.0
    }

    pub fn r#type(&self) -> &RustType {
        &self.name_type.1
    }

    pub fn constants(&self) -> &[(String, String)] {
        &self.constants[..]
    }

    pub fn with_constants(mut self, constants: Vec<(String, String)>) -> Self {
        self.constants = constants;
        self
    }
}

impl TagProperty for Field {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag);
    }

    fn reset_tag(&mut self) {
        self.tag = None;
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Enumeration<T> {
    variants: Vec<T>,
    tag: Option<Tag>,
    extended_after_index: Option<usize>,
}

impl<T> From<Vec<T>> for Enumeration<T> {
    fn from(variants: Vec<T>) -> Self {
        Enumeration {
            variants,
            tag: None,
            extended_after_index: None,
        }
    }
}

impl<T> Enumeration<T> {
    pub fn with_extension_after(mut self, extension_after: Option<usize>) -> Self {
        self.extended_after_index = extension_after;
        self
    }

    pub fn len(&self) -> usize {
        self.variants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    pub fn variants(&self) -> impl Iterator<Item = &T> {
        self.variants.iter()
    }

    pub fn extension_after_index(&self) -> Option<usize> {
        self.extended_after_index
    }

    pub fn extension_after_variant(&self) -> Option<&T> {
        self.extended_after_index
            .and_then(|index| self.variants.get(index))
    }

    pub fn is_extensible(&self) -> bool {
        self.extended_after_index.is_some()
    }
}

impl<T> TagProperty for Enumeration<T> {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag);
    }

    fn reset_tag(&mut self) {
        self.tag = None;
    }
}

impl PlainEnum {
    pub fn from_names(names: impl Iterator<Item = impl ToString>) -> Self {
        Self::from(names.map(|n| n.to_string()).collect::<Vec<_>>())
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct DataVariant {
    name_type: (String, RustType),
    tag: Option<Tag>,
}

impl DataVariant {
    pub fn from_name_type<T: ToString>(name: T, r#type: RustType) -> Self {
        Self {
            name_type: (name.to_string(), r#type),
            tag: None,
        }
    }

    pub fn fallback_representation(&self) -> &(String, RustType) {
        &self.name_type
    }

    pub fn name(&self) -> &str {
        &self.name_type.0
    }

    pub fn r#type(&self) -> &RustType {
        &self.name_type.1
    }
}

impl TagProperty for DataVariant {
    fn tag(&self) -> Option<Tag> {
        self.tag
    }

    fn set_tag(&mut self, tag: Tag) {
        self.tag = Some(tag);
    }

    fn reset_tag(&mut self) {
        self.tag = None;
    }
}

impl Model<Rust> {
    pub fn convert_asn_to_rust(asn_model: &Model<Asn>, scope: &[&Model<Asn>]) -> Model<Rust> {
        let mut model = Model {
            name: rust_module_name(&asn_model.name),
            oid: asn_model.oid.clone(),
            imports: asn_model
                .imports
                .iter()
                .map(|i| Import {
                    what: i.what.iter().map(|w| rust_struct_or_enum_name(w)).collect(),
                    from: rust_module_name(&i.from),
                    from_oid: i.from_oid.clone(),
                })
                .collect(),
            definitions: Vec::with_capacity(asn_model.definitions.len()),
            value_references: Vec::with_capacity(asn_model.definitions.len()),
        };
        for Definition(name, asn) in &asn_model.definitions {
            let rust_name = rust_struct_or_enum_name(name);
            let mut ctxt = Context {
                resolver: TagResolver {
                    model: asn_model,
                    scope,
                },
                target: &mut model.definitions,
            };
            Self::definition_to_rust(&rust_name, &asn.r#type, asn.tag, &mut ctxt);
        }
        model
    }

    /// Converts the given `Asn` value to `Rust`, adding new `Defintion`s as
    /// necessary (inlined types cannot be represented in rust and thus need to
    /// be extracted to their own types).
    /// The returned value is what shall be used to reference to the definition
    /// and can therefore be used to be inserted in the parent element.
    ///
    /// The name is expected in a valid and rusty way
    pub fn definition_to_rust(name: &str, asn: &AsnType, tag: Option<Tag>, ctxt: &mut Context<'_>) {
        match asn {
            AsnType::Boolean
            | AsnType::String(..)
            | AsnType::OctetString(_)
            | AsnType::BitString(_) => {
                let rust_type = Self::definition_type_to_rust_type(name, asn, tag, ctxt);
                ctxt.add_definition(Definition(
                    name.to_string(),
                    Rust::tuple_struct_from_type(rust_type).with_tag_opt(tag),
                ));
            }
            AsnType::TypeReference(_, tag) => {
                let rust_type = Self::definition_type_to_rust_type(name, asn, *tag, ctxt);
                ctxt.add_definition(Definition(
                    name.to_string(),
                    Rust::tuple_struct_from_type(rust_type).with_tag_opt(*tag),
                ));
            }

            me @ AsnType::Integer(_) => {
                let rust_type = Self::definition_type_to_rust_type(name, asn, tag, ctxt);
                let constants = Self::asn_constants_to_rust_constants(me);
                ctxt.add_definition(Definition(
                    name.into(),
                    Rust::TupleStruct {
                        r#type: rust_type,
                        tag,
                        constants,
                    },
                ));
            }

            AsnType::Optional(inner) => {
                let inner = RustType::Option(Box::new(Self::definition_type_to_rust_type(
                    name, inner, tag, ctxt,
                )));
                ctxt.add_definition(Definition(
                    name.into(),
                    Rust::tuple_struct_from_type(inner).with_tag_opt(tag),
                ))
            }

            AsnType::Sequence(ComponentTypeList {
                fields,
                extension_after,
            }) => {
                let fields = Self::asn_fields_to_rust_fields(name, fields, ctxt);
                ctxt.add_definition(Definition(
                    name.into(),
                    Rust::Struct {
                        ordering: EncodingOrdering::Keep,
                        fields,
                        tag,
                        extension_after: *extension_after,
                    },
                ));
            }

            AsnType::Set(ComponentTypeList {
                fields,
                extension_after,
            }) => {
                let fields = Self::asn_fields_to_rust_fields(name, fields, ctxt);
                ctxt.add_definition(Definition(
                    name.into(),
                    Rust::Struct {
                        ordering: EncodingOrdering::Sort,
                        fields,
                        tag,
                        extension_after: *extension_after,
                    },
                ));
            }

            AsnType::SequenceOf(asn, size) => {
                let inner = RustType::Vec(
                    Box::new(Self::definition_type_to_rust_type(name, asn, tag, ctxt)),
                    *size,
                    EncodingOrdering::Keep,
                );
                ctxt.add_definition(Definition(name.into(), Rust::tuple_struct_from_type(inner)));
            }

            AsnType::SetOf(asn, size) => {
                let inner = RustType::Vec(
                    Box::new(Self::definition_type_to_rust_type(name, asn, tag, ctxt)),
                    *size,
                    EncodingOrdering::Sort,
                );
                ctxt.add_definition(Definition(
                    name.into(),
                    Rust::tuple_struct_from_type(inner).with_tag_opt(tag),
                ));
            }

            AsnType::Choice(choice) => {
                let mut enumeration = Enumeration {
                    variants: Vec::with_capacity(choice.len()),
                    tag,
                    extended_after_index: choice.extension_after_index(),
                };

                for ChoiceVariant { name, r#type, tag } in choice.variants() {
                    let rust_name = format!("{}{}", name, rust_struct_or_enum_name(&name));
                    let rust_role =
                        Self::definition_type_to_rust_type(&rust_name, &r#type, *tag, ctxt);
                    let rust_field_name = rust_variant_name(&name);
                    enumeration.variants.push(
                        DataVariant::from_name_type(rust_field_name, rust_role).with_tag_opt(*tag),
                    );
                }

                ctxt.add_definition(Definition(name.into(), Rust::DataEnum(enumeration)));
            }

            AsnType::Enumerated(enumerated) => {
                let mut rust_enum = Enumeration {
                    variants: Vec::with_capacity(enumerated.len()),
                    tag,
                    extended_after_index: enumerated.extension_after_index(),
                };

                for variant in enumerated.variants() {
                    rust_enum.variants.push(rust_variant_name(variant.name()));
                }

                ctxt.add_definition(Definition(name.into(), Rust::Enum(rust_enum)));
            }
        }
    }

    fn asn_fields_to_rust_fields(
        name: &str,
        fields: &[crate::model::Field<Asn>],
        ctxt: &mut Context<'_>,
    ) -> Vec<Field> {
        let mut rust_fields = Vec::with_capacity(fields.len());

        for field in fields.iter() {
            let rust_name = format!("{}{}", name, rust_struct_or_enum_name(&field.name));
            let tag = field.role.tag;
            let rust_role =
                Self::definition_type_to_rust_type(&rust_name, &field.role.r#type, tag, ctxt);
            let rust_field_name = rust_field_name(&field.name);
            let constants = Self::asn_constants_to_rust_constants(&field.role.r#type);
            rust_fields.push(
                RustField::from_name_type(rust_field_name, rust_role)
                    .with_constants(constants)
                    .with_tag_opt(tag),
            );
        }

        rust_fields
    }

    pub fn asn_constants_to_rust_constants(asn: &AsnType) -> Vec<(String, String)> {
        match asn {
            AsnType::Integer(integer) => integer
                .constants
                .iter()
                .map(|(name, value)| (rust_module_name(name).to_uppercase(), format!("{}", value)))
                .collect(),
            AsnType::BitString(bitstring) => bitstring
                .constants
                .iter()
                .map(|(name, value)| (rust_module_name(name).to_uppercase(), format!("{}", value)))
                .collect(),

            Type::Boolean
            | Type::String(..)
            | Type::OctetString(_)
            | Type::Optional(_)
            | Type::Sequence(_)
            | Type::SequenceOf(..)
            | Type::Set(_)
            | Type::SetOf(..)
            | Type::Enumerated(_)
            | Type::Choice(_)
            | Type::TypeReference(_, _) => Vec::default(),
        }
    }

    pub fn definition_type_to_rust_type(
        name: &str,
        asn: &AsnType,
        tag: Option<Tag>,
        ctxt: &mut Context<'_>,
    ) -> RustType {
        match asn {
            AsnType::Boolean => RustType::Bool,
            AsnType::Integer(int) if int.range.extensible() => {
                match (int.range.min(), int.range.max()) {
                    (None, None)
                    | (Some(0), None)
                    | (Some(0), Some(i64::MAX))
                    | (None, Some(i64::MAX)) => RustType::U64(Range(None, None, true)),
                    (min, max) if min.unwrap_or_default() >= 0 && max.unwrap_or_default() >= 0 => {
                        RustType::U64(Range(min.map(|v| v as u64), max.map(|v| v as u64), true))
                    }
                    (min, max) => RustType::I64(Range(
                        min.unwrap_or_else(i64::min_value),
                        max.unwrap_or_else(i64::max_value),
                        true,
                    )),
                }
            }
            AsnType::Integer(int) => {
                match (int.range.min(), int.range.max()) {
                    (None, None)
                    | (Some(0), None)
                    | (Some(0), Some(i64::MAX))
                    | (None, Some(i64::MAX)) => RustType::U64(Range(None, None, false)),
                    (min, max) => {
                        let min = min.unwrap_or_default();
                        let max = max.unwrap_or(i64::MAX);
                        if min >= 0 {
                            match max as u64 {
                                m if m <= U8_MAX => RustType::U8(Range::inclusive(min as u8, max as u8)),
                                m if m <= U16_MAX => RustType::U16(Range::inclusive(min as u16, max as u16)),
                                m if m <= U32_MAX => RustType::U32(Range::inclusive(min as u32, max as u32)),
                                _/*m if m <= U64_MAX*/ => RustType::U64(Range::inclusive(Some(min as u64), Some(max as u64))),
                                //_ => panic!("This should never happen, since max (as u64 frm i64) cannot be greater than U64_MAX")
                            }
                        } else {
                            // i32 => -2147483648    to    2147483647  --\
                            //        -2147483648 + 1   = -2147483647    | same
                            //    abs(-2147483648 + 1)  =  2147483647  --/
                            let max_amplitude = (min + 1).abs().max(max);
                            match max_amplitude {
                                _ if max_amplitude <= I8_MAX => RustType::I8(Range::inclusive(min as i8, max as i8)),
                                _ if max_amplitude <= I16_MAX => RustType::I16(Range::inclusive(min as i16, max as i16)),
                                _ if max_amplitude <= I32_MAX => RustType::I32(Range::inclusive(min as i32, max as i32)),
                                _/*if max_amplitude <= I64_MAX*/ => RustType::I64(Range::inclusive(min as i64, max as i64)),
                                //_ => panic!("This should never happen, since max (being i64) cannot be greater than I64_MAX")
                            }
                        }
                    }
                }
            }

            AsnType::String(size, charset) => RustType::String(*size, *charset),
            AsnType::OctetString(size) => RustType::VecU8(*size),
            AsnType::BitString(bitstring) => RustType::BitVec(bitstring.size),
            Type::Optional(inner) => {
                RustType::Option(Box::new(Self::definition_type_to_rust_type(
                    name,
                    inner,
                    tag.or_else(|| ctxt.resolver().resolve_no_default(&**inner)),
                    ctxt,
                )))
            }
            AsnType::SequenceOf(asn, size) => RustType::Vec(
                Box::new(Self::definition_type_to_rust_type(
                    name,
                    asn,
                    tag.or_else(|| ctxt.resolver().resolve_no_default(&**asn)),
                    ctxt,
                )),
                *size,
                EncodingOrdering::Keep,
            ),
            AsnType::SetOf(asn, size) => RustType::Vec(
                Box::new(Self::definition_type_to_rust_type(
                    name,
                    asn,
                    tag.or_else(|| ctxt.resolver().resolve_no_default(&**asn)),
                    ctxt,
                )),
                *size,
                EncodingOrdering::Sort,
            ),
            ty @ AsnType::Sequence(_)
            | ty @ AsnType::Set(_)
            | ty @ AsnType::Enumerated(_)
            | ty @ AsnType::Choice(_) => {
                let name = rust_struct_or_enum_name(name);
                Self::definition_to_rust(&name, asn, tag, ctxt);
                RustType::Complex(name, tag.or_else(|| ctxt.resolver().resolve_type_tag(ty)))
            }
            AsnType::TypeReference(name, tag) => RustType::Complex(
                name.clone(),
                tag.clone().or_else(|| ctxt.resolver().resolve_tag(name)),
            ),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
pub fn rust_field_name(name: &str) -> String {
    rust_module_name(name)
}

#[allow(clippy::module_name_repetitions)]
pub fn rust_variant_name(name: &str) -> String {
    let mut out = String::new();
    let mut next_upper = true;
    for c in name.chars() {
        if next_upper {
            out.push_str(&c.to_uppercase().to_string());
            next_upper = false;
        } else if c == '-' {
            next_upper = true;
        } else {
            out.push(c);
        }
    }
    out
}

#[allow(clippy::module_name_repetitions)]
pub fn rust_struct_or_enum_name(name: &str) -> String {
    rust_variant_name(name)
}

#[allow(clippy::module_name_repetitions)]
pub fn rust_module_name(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lowered = false;
    let mut prev_alphabetic = false;
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        let mut lowered = false;
        let alphabetic = c.is_alphabetic();
        if c.is_uppercase() {
            if !out.is_empty() && prev_alphabetic {
                if !prev_lowered {
                    out.push('_');
                } else if let Some(next) = chars.peek() {
                    if next.is_lowercase() {
                        out.push('_');
                    }
                }
            }
            lowered = true;
            out.push_str(&c.to_lowercase().to_string());
        } else if c == '-' {
            out.push('_');
        } else {
            out.push(c);
        }
        prev_lowered = lowered;
        prev_alphabetic = alphabetic;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::tests::*;
    use crate::model::{Choice, Enumerated, EnumeratedVariant, Field, Tag, Type};
    use crate::parser::Tokenizer;

    #[test]
    fn test_simple_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Tokenizer::default().parse(SIMPLE_INTEGER_STRUCT_ASN))
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Simple".into(),
                Rust::struct_from_fields(vec![
                    RustField::from_name_type("small", RustType::U8(Range::inclusive(0, 255))),
                    RustField::from_name_type("bigger", RustType::U16(Range::inclusive(0, 65535))),
                    RustField::from_name_type("negative", RustType::I16(Range::inclusive(-1, 255))),
                    RustField::from_name_type(
                        "unlimited",
                        RustType::Option(Box::new(RustType::U64(Range::none()))),
                    ),
                ]),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_inline_asn_enumerated_represented_correctly_as_rust_model() {
        let modle_rust = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_ENUM))
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", modle_rust.name);
        assert_eq!(true, modle_rust.imports.is_empty());
        assert_eq!(2, modle_rust.definitions.len());
        assert_eq!(
            Definition(
                "WoahDecision".into(),
                Rust::Enum(
                    vec![
                        "ABORT".into(),
                        "RETURN".into(),
                        "CONFIRM".into(),
                        "MAYDAY".into(),
                        "THE_CAKE_IS_A_LIE".into()
                    ]
                    .into()
                ),
            ),
            modle_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "decision",
                    RustType::Option(Box::new(RustType::Complex(
                        "WoahDecision".into(),
                        Some(Tag::DEFAULT_ENUMERATED)
                    ))),
                )])
            ),
            modle_rust.definitions[1]
        );
    }

    #[test]
    fn test_inline_asn_sequence_of_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE_OF))
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(3, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "Ones".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::U8(Range::inclusive(0, 1))),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "NestedOnes".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::Vec(
                        Box::new(RustType::U8(Range::inclusive(0, 1))),
                        Size::Any,
                        EncodingOrdering::Keep
                    )),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[1]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::struct_from_fields(vec![
                    RustField::from_name_type(
                        "also_ones",
                        RustType::Vec(
                            Box::new(RustType::U8(Range::inclusive(0, 1))),
                            Size::Any,
                            EncodingOrdering::Keep
                        ),
                    ),
                    RustField::from_name_type(
                        "nesteds",
                        RustType::Vec(
                            Box::new(RustType::Vec(
                                Box::new(RustType::U8(Range::inclusive(0, 1))),
                                Size::Any,
                                EncodingOrdering::Keep
                            )),
                            Size::Any,
                            EncodingOrdering::Keep
                        ),
                    ),
                    RustField::from_name_type(
                        "optionals",
                        RustType::Option(Box::new(RustType::Vec(
                            Box::new(RustType::Vec(
                                Box::new(RustType::U64(Range::none())),
                                Size::Any,
                                EncodingOrdering::Keep
                            )),
                            Size::Any,
                            EncodingOrdering::Keep
                        ))),
                    )
                ]),
            ),
            model_rust.definitions[2]
        );
    }

    #[test]
    fn test_inline_asn_choice_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_CHOICE))
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(5, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "This".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::U8(Range::inclusive(0, 1))),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "That".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::Vec(
                        Box::new(RustType::U8(Range::inclusive(0, 1))),
                        Size::Any,
                        EncodingOrdering::Keep
                    )),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[1]
        );
        assert_eq!(
            Definition(
                "Neither".into(),
                Rust::Enum(vec!["ABC".into(), "DEF".into(),].into()),
            ),
            model_rust.definitions[2]
        );
        assert_eq!(
            Definition(
                "WoahDecision".into(),
                Rust::DataEnum(
                    vec![
                        DataVariant::from_name_type(
                            "This",
                            RustType::Complex("This".into(), Some(Tag::DEFAULT_SEQUENCE_OF))
                        ),
                        DataVariant::from_name_type(
                            "That",
                            RustType::Complex("That".into(), Some(Tag::DEFAULT_SEQUENCE_OF))
                        ),
                        DataVariant::from_name_type(
                            "Neither",
                            RustType::Complex("Neither".into(), Some(Tag::DEFAULT_ENUMERATED))
                        ),
                    ]
                    .into()
                )
            ),
            model_rust.definitions[3]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "decision",
                    RustType::Complex("WoahDecision".into(), Some(Tag::DEFAULT_ENUMERATED)),
                )])
            ),
            model_rust.definitions[4]
        );
    }

    #[test]
    fn test_inline_asn_sequence_represented_correctly_as_rust_model() {
        let model_rust = Model::try_from(Tokenizer::default().parse(INLINE_ASN_WITH_SEQUENCE))
            .unwrap()
            .to_rust();

        assert_eq!("simple_schema", model_rust.name);
        assert_eq!(true, model_rust.imports.is_empty());
        assert_eq!(2, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "WoahComplex".into(),
                Rust::struct_from_fields(vec![
                    RustField::from_name_type("ones", RustType::U8(Range::inclusive(0, 1))),
                    RustField::from_name_type(
                        "list_ones",
                        RustType::Vec(
                            Box::new(RustType::U8(Range::inclusive(0, 1))),
                            Size::Any,
                            EncodingOrdering::Keep
                        ),
                    ),
                    RustField::from_name_type(
                        "optional_ones",
                        RustType::Option(Box::new(RustType::Vec(
                            Box::new(RustType::U8(Range::inclusive(0, 1,))),
                            Size::Any,
                            EncodingOrdering::Keep
                        ))),
                    ),
                ]),
            ),
            model_rust.definitions[0]
        );
        assert_eq!(
            Definition(
                "Woah".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "complex",
                    RustType::Option(Box::new(RustType::Complex(
                        "WoahComplex".into(),
                        Some(Tag::DEFAULT_SEQUENCE)
                    ))),
                )]),
            ),
            model_rust.definitions[1]
        );
    }

    #[test]
    fn test_simple_enum() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "SimpleEnumTest".into(),
            AsnType::Enumerated(Enumerated::from_names(
                ["Bernd", "Das-Verdammte", "Brooot"].iter(),
            ))
            .untagged(),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "SimpleEnumTest".into(),
                Rust::Enum(vec!["Bernd".into(), "DasVerdammte".into(), "Brooot".into(),].into()),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_choice_simple() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "SimpleChoiceTest".into(),
            AsnType::choice_from_variants(vec![
                ChoiceVariant::name_type("bernd-das-brot", AsnType::unconstrained_utf8string()),
                ChoiceVariant::name_type("nochSoEinBrot", AsnType::unconstrained_octetstring()),
            ])
            .untagged(),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "SimpleChoiceTest".into(),
                Rust::DataEnum(
                    vec![
                        DataVariant::from_name_type(
                            "BerndDasBrot",
                            RustType::String(Size::Any, Charset::Utf8),
                        ),
                        DataVariant::from_name_type("NochSoEinBrot", RustType::VecU8(Size::Any)),
                    ]
                    .into()
                ),
            ),
            model_rust.definitions[0]
        )
    }

    #[test]
    fn test_choice_list_and_nested_list() {
        let mut model_asn = Model::default();
        model_asn.definitions.push(Definition(
            "ListChoiceTestWithNestedList".into(),
            AsnType::choice_from_variants(vec![
                ChoiceVariant::name_type(
                    "normal-List",
                    AsnType::SequenceOf(Box::new(AsnType::unconstrained_utf8string()), Size::Any),
                ),
                ChoiceVariant::name_type(
                    "NESTEDList",
                    AsnType::SequenceOf(
                        Box::new(AsnType::SequenceOf(
                            Box::new(AsnType::unconstrained_octetstring()),
                            Size::Any,
                        )),
                        Size::Any,
                    ),
                ),
            ])
            .untagged(),
        ));

        let model_rust = model_asn.to_rust();

        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "ListChoiceTestWithNestedList".into(),
                Rust::DataEnum(
                    vec![
                        DataVariant::from_name_type(
                            "NormalList",
                            RustType::Vec(
                                Box::new(RustType::String(Size::Any, Charset::Utf8)),
                                Size::Any,
                                EncodingOrdering::Keep
                            ),
                        ),
                        DataVariant::from_name_type(
                            "NESTEDList",
                            RustType::Vec(
                                Box::new(RustType::Vec(
                                    Box::new(RustType::VecU8(Size::Any)),
                                    Size::Any,
                                    EncodingOrdering::Keep
                                )),
                                Size::Any,
                                EncodingOrdering::Keep
                            ),
                        ),
                    ]
                    .into()
                ),
            ),
            model_rust.definitions[0]
        )
    }

    #[test]
    fn test_tuple_list() {
        let mut model_asn = Model::default();
        model_asn.name = "TupleTestModel".into();
        model_asn.definitions.push(Definition(
            "TupleTest".into(),
            AsnType::SequenceOf(Box::new(AsnType::unconstrained_utf8string()), Size::Any)
                .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("tuple_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "TupleTest".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::String(Size::Any, Charset::Utf8)),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_nested_tuple_list() {
        let mut model_asn = Model::default();
        model_asn.name = "TupleTestModel".into();
        model_asn.definitions.push(Definition(
            "NestedTupleTest".into(),
            AsnType::SequenceOf(
                Box::new(AsnType::SequenceOf(
                    Box::new(AsnType::unconstrained_utf8string()),
                    Size::Any,
                )),
                Size::Any,
            )
            .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("tuple_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "NestedTupleTest".into(),
                Rust::tuple_struct_from_type(RustType::Vec(
                    Box::new(RustType::Vec(
                        Box::new(RustType::String(Size::Any, Charset::Utf8)),
                        Size::Any,
                        EncodingOrdering::Keep
                    )),
                    Size::Any,
                    EncodingOrdering::Keep
                )),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_optional_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "OptionalStructListTestModel".into();
        model_asn.definitions.push(Definition(
            "OptionalStructListTest".into(),
            AsnType::sequence_from_fields(vec![Field {
                name: "strings".into(),
                role: AsnType::SequenceOf(Box::new(AsnType::unconstrained_utf8string()), Size::Any)
                    .optional()
                    .untagged(),
            }])
            .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("optional_struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "OptionalStructListTest".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "strings",
                    RustType::Option(Box::new(RustType::Vec(
                        Box::new(RustType::String(Size::Any, Charset::Utf8)),
                        Size::Any,
                        EncodingOrdering::Keep
                    ))),
                )]),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "StructListTestModel".into();
        model_asn.definitions.push(Definition(
            "StructListTest".into(),
            AsnType::sequence_from_fields(vec![Field {
                name: "strings".into(),
                role: AsnType::SequenceOf(Box::new(AsnType::unconstrained_utf8string()), Size::Any)
                    .untagged(),
            }])
            .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "StructListTest".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "strings",
                    RustType::Vec(
                        Box::new(RustType::String(Size::Any, Charset::Utf8)),
                        Size::Any,
                        EncodingOrdering::Keep
                    ),
                )]),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    fn test_nested_list_in_struct() {
        let mut model_asn = Model::default();
        model_asn.name = "NestedStructListTestModel".into();
        model_asn.definitions.push(Definition(
            "NestedStructListTest".into(),
            AsnType::sequence_from_fields(vec![Field {
                name: "strings".into(),
                role: AsnType::SequenceOf(
                    Box::new(AsnType::SequenceOf(
                        Box::new(AsnType::unconstrained_utf8string()),
                        Size::Any,
                    )),
                    Size::Any,
                )
                .untagged(),
            }])
            .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("nested_struct_list_test_model", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(1, model_rust.definitions.len());
        assert_eq!(
            Definition(
                "NestedStructListTest".into(),
                Rust::struct_from_fields(vec![RustField::from_name_type(
                    "strings",
                    RustType::Vec(
                        Box::new(RustType::Vec(
                            Box::new(RustType::String(Size::Any, Charset::Utf8)),
                            Size::Any,
                            EncodingOrdering::Keep
                        )),
                        Size::Any,
                        EncodingOrdering::Keep
                    ),
                )]),
            ),
            model_rust.definitions[0]
        );
    }

    #[test]
    pub fn test_extensible_enum() {
        let mut model_asn = Model::default();
        model_asn.name = "ExtensibleEnum".to_string();
        model_asn.definitions.push(Definition(
            "Extensible".to_string(),
            AsnType::Enumerated(Enumerated {
                variants: vec![
                    "abc".into(),
                    "def".into(),
                    EnumeratedVariant::from_name_number("ghi", 42),
                ],
                extension_after: Some(2),
            })
            .untagged(),
        ));
        let model_rust = model_asn.to_rust();
        assert_eq!("extensible_enum", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(
            &[Definition(
                "Extensible".into(),
                Rust::Enum(
                    PlainEnum::from_names(["Abc", "Def", "Ghi"].iter())
                        .with_extension_after(Some(2))
                ),
            )],
            &model_rust.definitions[..]
        );
    }

    #[test]
    pub fn test_extensible_choice() {
        let mut model_asn = Model::default();
        model_asn.name = "ExtensibleChoice".to_string();
        model_asn.definitions.push(Definition(
            "Extensible".to_string(),
            AsnType::Choice(Choice {
                variants: vec![
                    ChoiceVariant::name_type("abc", Type::unconstrained_octetstring()),
                    ChoiceVariant::name_type("def", Type::unconstrained_integer()),
                    ChoiceVariant {
                        name: "ghi".to_string(),
                        tag: Some(Tag::Universal(4)),
                        r#type: Type::Boolean,
                    },
                ],
                extension_after: Some(2),
            })
            .untagged(),
        ));

        let model_rust = model_asn.to_rust();
        assert_eq!("extensible_choice", model_rust.name);
        assert_eq!(model_asn.imports, model_rust.imports);
        assert_eq!(
            &[Definition(
                "Extensible".into(),
                Rust::DataEnum(
                    DataEnum::from(vec![
                        DataVariant::from_name_type("Abc".to_string(), RustType::VecU8(Size::Any)),
                        DataVariant::from_name_type(
                            "Def".to_string(),
                            RustType::U64(Range::none()),
                        ),
                        DataVariant::from_name_type("Ghi".to_string(), RustType::Bool)
                            .with_tag(Tag::Universal(4)),
                    ])
                    .with_extension_after(Some(2))
                ),
            )],
            &model_rust.definitions[..]
        );
    }
}
