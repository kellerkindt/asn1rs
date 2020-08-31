use crate::gen::rust::GeneratorSupplement;
use crate::gen::rust::RustCodeGenerator;
use crate::model::rust::{DataEnum, Enumeration};
use crate::model::rust::{Field, PlainEnum};
use crate::model::Definition;
use crate::model::Rust;
use crate::model::RustType;
use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

#[allow(clippy::module_name_repetitions)]
pub struct UperSerializer;

impl GeneratorSupplement<Rust> for UperSerializer {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1rs::io::uper", Self::CODEC);
        scope.import(
            "asn1rs::io::uper",
            &format!("Error as {}Error", Self::CODEC),
        );
        scope.import(
            "asn1rs::io::uper",
            &format!("Reader as {}Reader", Self::CODEC),
        );
        scope.import(
            "asn1rs::io::uper",
            &format!("Writer as {}Writer", Self::CODEC),
        );
    }

    fn impl_supplement(&self, scope: &mut Scope, definition: &Definition<Rust>) {
        let serializable_implementation = Self::new_uper_serializable_impl(scope, &definition.0);
        Self::impl_read_fn(Self::new_read_fn(serializable_implementation), definition);
        Self::impl_write_fn(Self::new_write_fn(serializable_implementation), definition);
    }
}

impl UperSerializer {
    const CODEC: &'static str = "Uper";

    fn new_uper_serializable_impl<'a>(scope: &'a mut Scope, impl_for: &str) -> &'a mut Impl {
        RustCodeGenerator::new_serializable_impl(scope, impl_for, Self::CODEC)
    }

    fn new_read_fn(implementation: &mut Impl) -> &mut Function {
        RustCodeGenerator::new_read_fn(implementation, Self::CODEC)
    }

    fn impl_read_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct {
                r#type: aliased, ..
            } => {
                Self::impl_read_fn_for_tuple_struct(function, name, aliased);
            }
            Rust::Struct {
                fields,
                extension_after: _,
            } => {
                for field in fields.iter() {
                    Self::impl_read_fn_header_for_type(function, field.name(), field.r#type());
                }
                Self::impl_read_fn_for_struct(function, fields);
            }
            Rust::Enum(r_enum) => {
                Self::impl_read_fn_for_enum(function, name, r_enum);
            }
            Rust::DataEnum(enumeration) => {
                Self::impl_read_fn_for_data_enum(function, name, enumeration);
            }
        };
    }

    fn impl_read_fn_for_tuple_struct(function: &mut Function, name: &str, aliased: &RustType) {
        Self::impl_read_fn_header_for_type(function, name, aliased);
        function.push_block({
            let mut block = Block::new(&format!("Ok({}(", name));
            Self::impl_read_fn_for_type(
                &mut block,
                &aliased.to_inner_type_string(),
                Some(Member::Instance("0".into(), !aliased.is_primitive(), false)),
                aliased,
            );
            block.after("))");
            block
        });
    }

    fn impl_read_fn_header_for_type(function: &mut Function, name: &str, aliased: &RustType) {
        if let RustType::Option(_) = aliased {
            function.line(&format!(
                "let {} = reader.read_bit()?;",
                RustCodeGenerator::rust_field_name(name, true)
            ));
        }
    }

    fn impl_read_fn_for_type(
        block: &mut Block,
        type_name: &str,
        field_name: Option<Member>,
        rust: &RustType,
    ) {
        match rust {
            RustType::Bool => {
                block.line("reader.read_bit()?");
            }
            RustType::U8(_)
            | RustType::I8(_)
            | RustType::U16(_)
            | RustType::I16(_)
            | RustType::U32(_)
            | RustType::I32(_)
            | RustType::I64(_) => {
                let prefix = Self::min_max_prefix(&field_name);
                block.line(format!(
                    "reader.read_int((i64::from(Self::{}min()), i64::from(Self::{}max())))? as {}",
                    prefix,
                    prefix,
                    rust.to_string(),
                ));
            }
            RustType::U64(range) => {
                if range
                    .min_max(u64::min_value, || i64::max_value() as u64)
                    .is_some()
                {
                    let prefix = Self::min_max_prefix(&field_name);
                    block.line(format!(
                        "reader.read_int((Self::{}min() as i64, Self::{}max() as i64))? as {}",
                        prefix,
                        prefix,
                        rust.to_string(),
                    ));
                } else {
                    block.line("reader.read_int_max_signed()? as _");
                }
            }
            RustType::String => {
                block.line("reader.read_utf8_string()?");
            }
            RustType::VecU8(_) => {
                block.line("reader.read_octet_string(None)?");
            }
            RustType::BitVec(_) => {
                block.line("reader.read_bitstring()?");
            }
            RustType::Vec(inner) => {
                block.line("let len = reader.read_length_determinant()?;");
                block.line("let mut values = Vec::with_capacity(len);");
                let mut for_block = Block::new("for _ in 0..len");
                for_block.push_block({
                    let mut inner_block = Block::new("values.push(");
                    Self::impl_read_fn_for_type(
                        &mut inner_block,
                        &inner.to_inner_type_string(),
                        Some(Member::Local(
                            field_name.map_or_else(|| "value".into(), |f| f.name().to_string()),
                            false,
                            false,
                        )),
                        inner,
                    );
                    inner_block.after(");");
                    inner_block
                });
                block.push_block(for_block);
                block.line("values");
            }
            RustType::Option(inner) => {
                let mut if_block = Block::new(&format!(
                    "if {}",
                    field_name
                        .clone()
                        .map_or_else(|| "value".into(), |f| f.name().to_string())
                ));
                let mut if_true_block = Block::new("Some(");
                Self::impl_read_fn_for_type(
                    &mut if_true_block,
                    &inner.to_inner_type_string(),
                    field_name,
                    inner,
                );
                if_true_block.after(")");
                if_block.push_block(if_true_block);
                let mut else_block = Block::new("else");
                else_block.line("None");
                block.push_block(if_block);
                block.push_block(else_block);
            }
            RustType::Complex(_inner) => {
                block.line(format!("{}::read_uper(reader)?", type_name));
            }
        };
    }

    fn min_max_prefix(field_name: &Option<Member>) -> String {
        if let Some(ref field_name) = field_name {
            if field_name.name().ne("0") {
                format!("{}_", field_name.name())
            } else {
                "value_".to_string()
            }
        } else {
            String::default()
        }
    }

    fn impl_read_fn_for_struct(function: &mut Function, fields: &[Field]) {
        function.line("let mut me = Self::default();");
        for field in fields {
            let mut block = Block::new(&format!(
                "me.{} = ",
                RustCodeGenerator::rust_field_name(field.name(), true)
            ));
            Self::impl_read_fn_for_type(
                &mut block,
                &field.r#type().to_inner_type_string(),
                Some(Member::Instance(field.name().to_string(), false, false)),
                field.r#type(),
            );
            block.after(";");
            function.push_block(block);
        }
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_enum(function: &mut Function, name: &str, r_enum: &PlainEnum) {
        if let Some(last_standard_index) = r_enum.extension_after_index() {
            function.line(format!(
                "let id = reader.read_choice_index_extensible({})? as i64;",
                last_standard_index + 1
            ));
        } else {
            function.line(format!(
                "let id = reader.read_int((0, {}))?;",
                r_enum.len() - 1
            ));
        }
        let mut block_match = Block::new("match id");
        for (i, variant) in r_enum.variants().enumerate() {
            block_match.line(format!("{} => Ok({}::{}),", i, name, variant));
        }
        block_match.line(format!(
            "_ => Err(UperError::ValueNotInRange(id, 0, {}))",
            r_enum.len() - 1
        ));
        function.push_block(block_match);
    }

    fn impl_read_fn_for_data_enum(function: &mut Function, name: &str, enumeration: &DataEnum) {
        if enumeration.len() > 1 {
            if let Some(last_standard_index) = enumeration.extension_after_index() {
                function.line(&format!(
                    "let variant = reader.read_choice_index_extensible({})? as i64;",
                    last_standard_index + 1
                ));
            } else {
                function.line(&format!(
                    "let variant = reader.read_int((0, {}))?;",
                    enumeration.len() - 1
                ));
            }
        } else {
            function.line("let variant = 0;");
        }
        let mut block = Block::new("match variant");
        for (i, variant) in enumeration.variants().enumerate() {
            let mut block_case = Block::new(&format!("{} => Ok({}::{}(", i, name, variant.name()));
            let var_name = RustCodeGenerator::rust_module_name(variant.name());

            if Self::is_extended_variant(enumeration, i) {
                block_case.line(
                    "let mut reader = reader.read_substring_with_length_determinant_prefix()?;",
                );
                block_case.line(format!(
                    "let reader = &mut reader as &mut dyn {}Reader;",
                    Self::CODEC
                ));
            }
            Self::impl_read_fn_for_type(
                &mut block_case,
                &variant.r#type().to_inner_type_string(),
                Some(Member::Local(
                    var_name,
                    false,
                    variant.r#type().is_primitive(),
                )),
                variant.r#type(),
            );

            block_case.after(")),");
            block.push_block(block_case);
        }
        let err_line = format!(
            "Err(UperError::ValueNotInRange(variant, 0, {}))",
            enumeration.len() - 1
        );
        if enumeration.is_extensible() {
            let mut block_default = Block::new("_ => ");
            block_default.line("// skip the content of the unknown variant");
            block_default.line("let _ = reader.read_substring_with_length_determinant_prefix()?;");
            block_default.line(err_line);
            block.push_block(block_default);
        } else {
            block.line(format!("_ => {}", err_line));
        }
        function.push_block(block);
    }

    fn new_write_fn(implementation: &mut Impl) -> &mut Function {
        RustCodeGenerator::new_write_fn(implementation, Self::CODEC)
    }

    fn impl_write_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct { r#type: inner, .. } => {
                Self::impl_write_fn_for_tuple_struct(function, inner);
            }
            Rust::Struct {
                fields,
                extension_after: _,
            } => {
                for field in fields.iter() {
                    Self::impl_write_fn_header_for_type(function, field.name(), field.r#type());
                }
                Self::impl_write_fn_for_struct(function, fields);
            }
            Rust::Enum(r_enum) => {
                Self::impl_write_fn_for_enum(function, name, r_enum);
            }
            Rust::DataEnum(enumeration) => {
                Self::impl_write_fn_for_data_enum(function, name, enumeration);
            }
        }
    }

    fn impl_write_fn_for_tuple_struct(function: &mut Function, aliased: &RustType) {
        Self::impl_write_fn_header_for_type(function, "self.0", aliased);
        function.push_block({
            let mut block = Block::new("");
            Self::impl_write_fn_for_type(
                &mut block,
                Some(Member::Instance("0".into(), !aliased.is_primitive(), false)),
                aliased,
            );
            block
        });
        function.line("Ok(())");
    }

    fn impl_write_fn_header_for_type(function: &mut Function, name: &str, aliased: &RustType) {
        if let RustType::Option(_) = aliased {
            function.line(&format!(
                "writer.write_bit(self.{}.is_some())?;",
                RustCodeGenerator::rust_field_name(name, true)
            ));
        }
    }

    fn impl_write_fn_for_type(block: &mut Block, field_name: Option<Member>, rust: &RustType) {
        match rust {
            RustType::Bool => {
                block.line(format!(
                    "writer.write_bit({})?;",
                    field_name
                        .as_ref()
                        .map_or_else(|| "value".into(), |f| f.to_string()),
                ));
            }
            RustType::U8(_)
            | RustType::I8(_)
            | RustType::U16(_)
            | RustType::I16(_)
            | RustType::U32(_)
            | RustType::I32(_)
            | RustType::I64(_) => {
                let prefix = Self::min_max_prefix(&field_name);
                block.line(format!(
                    "writer.write_int(i64::from({}), (i64::from(Self::{}min()), i64::from(Self::{}max())))?;",
                    field_name
                        .as_ref()
                        .map_or_else(|| "value".into(), |f| f.to_string()),
                    prefix,
                    prefix,
                ));
            }
            RustType::U64(range) => {
                if range
                    .min_max(u64::min_value, || i64::max_value() as u64)
                    .is_some()
                {
                    let prefix = Self::min_max_prefix(&field_name);
                    block.line(format!(
                        "writer.write_int({} as i64, (Self::{}min() as i64, Self::{}max() as i64))?;",
                        field_name
                            .as_ref()
                            .map_or_else(|| "value".into(), |f| f.to_string()),
                        prefix,
                        prefix,
                    ));
                } else {
                    block.line(&format!(
                        "writer.write_int_max_signed({} as _)?;",
                        field_name.map_or_else(|| "value".into(), |f| f.to_string()),
                    ));
                }
            }
            RustType::String => {
                block.line(&format!(
                    "writer.write_utf8_string({})?;",
                    field_name.map_or_else(|| "value".into(), |f| f.to_string()),
                ));
            }
            RustType::VecU8(_) => {
                block.line(format!(
                    "writer.write_octet_string({}[..], None)?;",
                    field_name.map_or_else(|| "value".into(), |f| f.with_ref().to_string()),
                ));
            }
            RustType::BitVec(_) => {
                block.line(format!(
                    "writer.write_bitstring({})?;",
                    field_name.map_or_else(|| "value".into(), |f| f.with_ref().to_string()),
                ));
            }
            RustType::Vec(inner) => {
                block.line(format!(
                    "writer.write_length_determinant({}.len())?;",
                    field_name
                        .clone()
                        .map_or_else(|| "value".into(), |f| f.no_ref().to_string())
                ));
                let local_name = field_name
                    .as_ref()
                    .map(|f| f.name().to_string())
                    .filter(|name| name.ne("0"))
                    .unwrap_or_else(|| "value".into());
                let mut for_block = Block::new(&format!(
                    "for {} in {}{}",
                    local_name,
                    if field_name
                        .as_ref()
                        .map_or(false, |f| if let Member::Local(..) = f {
                            true
                        } else {
                            false
                        })
                    {
                        ""
                    } else {
                        "&"
                    },
                    field_name.map_or_else(|| "value".into(), |f| f.no_ref().to_string()),
                ));
                Self::impl_write_fn_for_type(
                    &mut for_block,
                    Some(Member::Local(local_name, false, inner.is_primitive())),
                    inner,
                );
                block.push_block(for_block);
            }
            RustType::Option(inner) => {
                let mut if_block = Block::new(&format!(
                    "if let Some({}{}) = {}",
                    if inner.is_primitive() { "" } else { "ref " },
                    field_name
                        .clone()
                        .map_or_else(|| "value".into(), |f| f.name().to_string()),
                    field_name
                        .clone()
                        .map_or_else(|| "value".into(), |f| f.to_string()),
                ));
                Self::impl_write_fn_for_type(
                    &mut if_block,
                    Some(Member::Local(
                        field_name.map_or_else(|| "value".into(), |f| f.name().to_string()),
                        false,
                        false,
                    )),
                    inner,
                );
                block.push_block(if_block);
            }
            RustType::Complex(_inner) => {
                block.line(format!(
                    "{}.write_uper(writer)?;",
                    &field_name.map_or_else(
                        || "value".into(),
                        |mut f| {
                            *f.name_mut() = RustCodeGenerator::rust_field_name(f.name(), true);
                            f.no_ref().to_string()
                        },
                    ),
                ));
            }
        }
    }
    fn impl_write_fn_for_struct(function: &mut Function, fields: &[Field]) {
        let mut block = Block::new("");
        for field in fields.iter() {
            Self::impl_write_fn_for_type(
                &mut block,
                Some(Member::Instance(
                    field.name().to_string(),
                    !field.r#type().clone().no_option().is_primitive(),
                    false,
                )),
                field.r#type(),
            );
        }
        function.push_block(block);
        function.line("Ok(())");
    }

    fn impl_write_fn_for_enum(function: &mut Function, name: &str, r_enum: &PlainEnum) {
        let mut block = Block::new("match self");
        for (i, variant) in r_enum.variants().enumerate() {
            if let Some(last_standard_index) = r_enum.extension_after_index() {
                block.line(format!(
                    "{}::{} => writer.write_choice_index_extensible({}, {})?,",
                    name,
                    &variant,
                    i,
                    last_standard_index + 1
                ));
            } else {
                block.line(format!(
                    "{}::{} => writer.write_int({}, (0, {}))?,",
                    name,
                    &variant,
                    i,
                    r_enum.len() - 1
                ));
            }
        }
        function.push_block(block);
        function.line("Ok(())");
    }

    fn impl_write_fn_for_data_enum(function: &mut Function, name: &str, enumeration: &DataEnum) {
        let mut block = Block::new("match self");
        for (i, variant) in enumeration.variants().enumerate() {
            let var_name = RustCodeGenerator::rust_module_name(variant.name());
            let mut block_case =
                Block::new(&format!("{}::{}({}) =>", name, variant.name(), var_name));

            if enumeration.len() > 1 {
                let is_extended_variant = Self::is_extended_variant(enumeration, i);

                if let Some(last_standard_index) = enumeration.extension_after_index() {
                    block_case.line(format!(
                        "writer.write_choice_index_extensible({}, {})?;",
                        i,
                        last_standard_index + 1
                    ));
                    if is_extended_variant {
                        let mut block_substring = Block::new(
                            "writer.write_substring_with_length_determinant_prefix(&|writer| ",
                        );
                        Self::impl_write_fn_for_type(
                            &mut block_substring,
                            Some(Member::Local(
                                var_name.clone(),
                                false,
                                variant.r#type().is_primitive(),
                            )),
                            variant.r#type(),
                        );
                        block_substring.line("Ok(())");
                        block_substring.after(")?;");
                        block_case.push_block(block_substring);
                    }
                } else {
                    block_case.line(format!(
                        "writer.write_int({}, (0, {}))?;",
                        i,
                        enumeration.len() - 1
                    ));
                }
                if !is_extended_variant {
                    Self::impl_write_fn_for_type(
                        &mut block_case,
                        Some(Member::Local(
                            var_name,
                            false,
                            variant.r#type().is_primitive(),
                        )),
                        variant.r#type(),
                    );
                }
            }
            block.push_block(block_case);
        }
        function.push_block(block);
        function.line("Ok(())");
    }

    fn is_extended_variant<T>(enumeration: &Enumeration<T>, variant: usize) -> bool {
        enumeration
            .extension_after_index()
            .map(|last| variant > last)
            .unwrap_or(false)
    }
}

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Member {
    Local(String, bool, bool),
    #[allow(dead_code)]
    Static(String, bool, bool),
    Instance(String, bool, bool),
}

impl Member {
    pub fn name(&self) -> &str {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            Member::Local(name, _, _) => name,
            Member::Static(name, _, _) => name,
            Member::Instance(name, _, _) => name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            Member::Local(ref mut name, _, _) => name,
            Member::Static(ref mut name, _, _) => name,
            Member::Instance(ref mut name, _, _) => name,
        }
    }

    #[allow(dead_code)]
    pub fn prefix_ref(&self) -> bool {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            Member::Local(_, prefix, _) => *prefix,
            Member::Static(_, prefix, _) => *prefix,
            Member::Instance(_, prefix, _) => *prefix,
        }
    }

    #[allow(dead_code)]
    pub fn prefix_deref(&self) -> bool {
        #[allow(clippy::match_same_arms)] // to have the same order as the original enum
        match self {
            Member::Local(_, _, prefix) => *prefix,
            Member::Static(_, _, prefix) => *prefix,
            Member::Instance(_, _, prefix) => *prefix,
        }
    }

    #[allow(clippy::match_same_arms)] // to have the same order as the original enum
    pub fn no_ref(mut self) -> Self {
        *match self {
            Member::Local(_, ref mut prefix, _) => prefix,
            Member::Static(_, ref mut prefix, _) => prefix,
            Member::Instance(_, ref mut prefix, _) => prefix,
        } = false;
        self
    }

    #[allow(clippy::match_same_arms)] // to have the same body on each
    pub fn with_ref(mut self) -> Self {
        *match self {
            Member::Local(_, ref mut prefix, _) => prefix,
            Member::Static(_, ref mut prefix, _) => prefix,
            Member::Instance(_, ref mut prefix, _) => prefix,
        } = true;
        self
    }

    #[allow(clippy::match_same_arms)] // to have the same body on each
    pub fn with_deref(mut self) -> Self {
        *match self {
            Member::Local(_, _, ref mut prefix) => prefix,
            Member::Static(_, _, ref mut prefix) => prefix,
            Member::Instance(_, _, ref mut prefix) => prefix,
        } = true;
        self
    }
}

impl ToString for Member {
    fn to_string(&self) -> String {
        match self {
            Member::Local(name, prefix_ref, prefix_deref) => format!(
                "{}{}{}",
                if *prefix_ref { "&" } else { "" },
                if *prefix_deref { "*" } else { "" },
                name.clone()
            ),
            Member::Static(name, prefix_ref, prefix_deref) => format!(
                "{}{}Self::{}",
                if *prefix_ref { "&" } else { "" },
                if *prefix_deref { "*" } else { "" },
                name.clone()
            ),
            Member::Instance(name, prefix_ref, prefix_deref) => format!(
                "{}{}self.{}",
                if *prefix_ref { "&" } else { "" },
                if *prefix_deref { "*" } else { "" },
                name.clone()
            ),
        }
    }
}
