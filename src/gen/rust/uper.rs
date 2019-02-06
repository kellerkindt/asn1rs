use crate::gen::rust::GeneratorSupplement;
use crate::gen::rust::RustCodeGenerator;
use crate::model::Definition;
use crate::model::Rust;
use crate::model::RustType;
use codegen::Block;
use codegen::Function;
use codegen::Impl;
use codegen::Scope;

pub struct UperSerializer;
impl GeneratorSupplement<Rust> for UperSerializer {
    fn add_imports(&self, scope: &mut Scope) {
        scope.import("asn1c::io::uper", Self::CODEC);
        scope.import("asn1c::io::uper", &format!("Error as {}Error", Self::CODEC));
        scope.import(
            "asn1c::io::uper",
            &format!("Reader as {}Reader", Self::CODEC),
        );
        scope.import(
            "asn1c::io::uper",
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
            Rust::TupleStruct(aliased) => {
                Self::impl_read_fn_for_tuple_struct(function, name, aliased);
            }
            Rust::Struct(fields) => {
                for (field_name, field_type) in fields.iter() {
                    Self::impl_read_fn_header_for_type(function, field_name, field_type);
                }
                Self::impl_read_fn_for_struct(function, fields);
            }
            Rust::Enum(variants) => {
                Self::impl_read_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(variants) => {
                Self::impl_read_fn_for_data_enum(function, name, &variants[..]);
            }
        };
    }

    fn impl_read_fn_for_tuple_struct(function: &mut Function, name: &str, aliased: &RustType) {
        Self::impl_read_fn_header_for_type(function, name, aliased);
        function.push_block({
            let mut block = Block::new(&format!("Ok({}(", name));
            Self::impl_read_fn_for_type(&mut block, &aliased.to_inner_type_string(), None, aliased);
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
            | RustType::U64(Some(_))
            | RustType::I64(_) => {
                block.line(format!(
                    "reader.read_int((i64::from(Self::{}min()), i64::from(Self::{}max())))? as {}",
                    if let Some(ref field_name) = field_name {
                        format!("{}_", field_name.name())
                    } else {
                        String::default()
                    },
                    if let Some(ref field_name) = field_name {
                        format!("{}_", field_name.name())
                    } else {
                        String::default()
                    },
                    rust.to_string(),
                ));
            }
            RustType::U64(None) => {
                block.line("reader.read_int_max()?");
            }
            RustType::String => {
                block.line("reader.read_utf8_string()?");
            }
            RustType::VecU8 => {
                block.line("reader.read_octet_string(None)?");
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
                            field_name
                                .clone()
                                .map(|f| f.name().to_string())
                                .unwrap_or_else(|| "value".into()),
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
                        .map(|f| f.name().to_string())
                        .unwrap_or_else(|| "value".into())
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

    fn impl_read_fn_for_struct(function: &mut Function, fields: &[(String, RustType)]) {
        function.line("let mut me = Self::default();");
        for (field_name, field_type) in fields.iter() {
            let mut block = Block::new(&format!(
                "me.{} = ",
                RustCodeGenerator::rust_field_name(field_name, true)
            ));
            Self::impl_read_fn_for_type(
                &mut block,
                &field_type.to_inner_type_string(),
                Some(Member::Instance(field_name.clone(), false, false)),
                field_type,
            );
            block.after(";");
            function.push_block(block);
        }
        function.line("Ok(me)");
    }

    fn impl_read_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        function.line(format!(
            "let id = reader.read_int((0, {}))?;",
            variants.len() - 1
        ));
        let mut block_match = Block::new("match id");
        for (i, variant) in variants.iter().enumerate() {
            block_match.line(format!("{} => Ok({}::{}),", i, name, variant));
        }
        block_match.line(format!(
            "_ => Err(UperError::ValueNotInRange(id, 0, {}))",
            variants.len() - 1
        ));
        function.push_block(block_match);
    }

    fn impl_read_fn_for_data_enum(
        function: &mut Function,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        if variants.len() > 1 {
            function.line(&format!(
                "let variant = reader.read_int((0, {}))?;",
                variants.len() - 1
            ));
        } else {
            function.line("let variant = 0");
        }
        let mut block = Block::new("match variant");
        for (i, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!("{} => Ok({}::{}(", i, name, variant));
            Self::impl_read_fn_for_type(&mut block_case, &role.to_inner_type_string(), None, role);
            block_case.after(")),");
            block.push_block(block_case);
        }
        block.line(format!(
            "_ => Err(UperError::ValueNotInRange(variant, 0, {}))",
            variants.len() - 1
        ));
        function.push_block(block);
    }

    fn new_write_fn(implementation: &mut Impl) -> &mut Function {
        RustCodeGenerator::new_write_fn(implementation, Self::CODEC)
    }

    fn impl_write_fn(function: &mut Function, Definition(name, rust): &Definition<Rust>) {
        match rust {
            Rust::TupleStruct(inner) => {
                Self::impl_write_fn_for_tuple_struct(function, inner);
            }
            Rust::Struct(fields) => {
                for (field_name, field_type) in fields.iter() {
                    Self::impl_write_fn_header_for_type(function, field_name, field_type);
                }
                Self::impl_write_fn_for_struct(function, fields);
            }
            Rust::Enum(variants) => {
                Self::impl_write_fn_for_enum(function, name, &variants[..]);
            }
            Rust::DataEnum(variants) => {
                Self::impl_write_fn_for_data_enum(function, name, &variants[..]);
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
                        .clone()
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                ));
            }
            RustType::U8(_)
            | RustType::I8(_)
            | RustType::U16(_)
            | RustType::I16(_)
            | RustType::U32(_)
            | RustType::I32(_)
            | RustType::U64(Some(_))
            | RustType::I64(_) => {
                block.line(format!(
                    "writer.write_int(i64::from({}), (i64::from(Self::{}min()), i64::from(Self::{}max())))?;",
                    field_name
                        .clone()
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                    if let Some(ref field_name) = field_name {
                        format!("{}_", field_name.name())
                    } else {
                        String::default()
                    },
                    if let Some(field_name) = field_name {
                        format!("{}_", field_name.name())
                    } else {
                        String::default()
                    },
                ));
            }
            RustType::U64(None) => {
                block.line(&format!(
                    "writer.write_int_max({})?;",
                    field_name
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                ));
            }
            RustType::String => {
                block.line(&format!(
                    "writer.write_utf8_string(&{})?;",
                    field_name
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                ));
            }
            RustType::VecU8 => {
                block.line(format!(
                    "writer.write_octet_string(&{}[..], None)?;",
                    field_name
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                ));
            }
            RustType::Vec(inner) => {
                block.line(format!(
                    "writer.write_length_determinant({}.len())?;",
                    field_name
                        .clone()
                        .map(|f| f.no_ref().to_string())
                        .unwrap_or_else(|| "value".into())
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
                        .map(|f| if let Member::Local(..) = f {
                            true
                        } else {
                            false
                        })
                        .unwrap_or(false)
                    {
                        ""
                    } else {
                        "&"
                    },
                    field_name
                        .map(|f| f.no_ref().to_string())
                        .unwrap_or_else(|| "value".into()),
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
                        .map(|f| f.name().to_string())
                        .unwrap_or_else(|| "value".into()),
                    field_name
                        .clone()
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "value".into()),
                ));
                Self::impl_write_fn_for_type(
                    &mut if_block,
                    Some(Member::Local(
                        field_name
                            .map(|f| f.name().to_string())
                            .unwrap_or_else(|| "value".into()),
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
                    &field_name
                        .map(|mut f| {
                            let name =
                                RustCodeGenerator::rust_field_name(f.name(), true).to_string();
                            *f.name_mut() = name;
                            f.to_string()
                        })
                        .unwrap_or_else(|| "value".into()),
                ));
            }
        }
    }
    fn impl_write_fn_for_struct(function: &mut Function, fields: &[(String, RustType)]) {
        let mut block = Block::new("");
        for (field_name, field_type) in fields.iter() {
            Self::impl_write_fn_for_type(
                &mut block,
                Some(Member::Instance(field_name.clone(), false, false)),
                field_type,
            );
        }
        function.push_block(block);
        function.line("Ok(())");
    }

    fn impl_write_fn_for_enum(function: &mut Function, name: &str, variants: &[String]) {
        let mut block = Block::new("match self");
        for (i, variant) in variants.iter().enumerate() {
            block.line(format!(
                "{}::{} => writer.write_int({}, (0, {}))?,",
                name,
                &variant,
                i,
                variants.len() - 1
            ));
        }
        function.push_block(block);
        function.line("Ok(())");
    }

    fn impl_write_fn_for_data_enum(
        function: &mut Function,
        name: &str,
        variants: &[(String, RustType)],
    ) {
        let mut block = Block::new("match self");
        for (i, (variant, role)) in variants.iter().enumerate() {
            let mut block_case = Block::new(&format!("{}::{}(value) =>", name, variant,));
            if variants.len() > 1 {
                block_case.line(format!(
                    "writer.write_int({}, (0, {}))?;",
                    i,
                    variants.len() - 1
                ));
            }
            Self::impl_write_fn_for_type(
                &mut block_case,
                Some(Member::Local("value".into(), false, role.is_primitive())),
                role,
            );
            block.push_block(block_case);
        }
        function.push_block(block);
        function.line("Ok(())");
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
        match self {
            Member::Local(name, _, _) => &name,
            Member::Static(name, _, _) => &name,
            Member::Instance(name, _, _) => &name,
        }
    }

    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Member::Local(ref mut name, _, _) => name,
            Member::Static(ref mut name, _, _) => name,
            Member::Instance(ref mut name, _, _) => name,
        }
    }

    #[allow(dead_code)]
    pub fn prefix_ref(&self) -> bool {
        match self {
            Member::Local(_, prefix, _) => *prefix,
            Member::Static(_, prefix, _) => *prefix,
            Member::Instance(_, prefix, _) => *prefix,
        }
    }

    #[allow(dead_code)]
    pub fn prefix_deref(&self) -> bool {
        match self {
            Member::Local(_, _, prefix) => *prefix,
            Member::Static(_, _, prefix) => *prefix,
            Member::Instance(_, _, prefix) => *prefix,
        }
    }

    pub fn no_ref(mut self) -> Self {
        *match self {
            Member::Local(_, ref mut prefix, _) => prefix,
            Member::Static(_, ref mut prefix, _) => prefix,
            Member::Instance(_, ref mut prefix, _) => prefix,
        } = false;
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
