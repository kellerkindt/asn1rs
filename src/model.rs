use parser::Token;

use std::vec::IntoIter;

#[derive(Debug)]
pub enum Error {
    ExpectedTextGot(String, String),
    ExpectedSeparatorGot(char, char),
    UnexpectedToken(Token),
    MissingModuleName,
    UnexpectedEndOfStream,
    InvalidRangeValue,
}

#[derive(Debug, Default, Clone)]
pub struct Model {
    pub name: String,
    pub imports: Vec<Import>,
    pub definitions: Vec<Definition>,
}

impl Model {
    pub fn try_from(value: Vec<Token>) -> Result<Self, Error> {
        let mut model = Model::default();
        let mut iter = value.into_iter();

        model.name = Self::read_name(&mut iter)?;
        Self::skip_after(&mut iter, &Token::Text("BEGIN".into()))?;

        while let Some(token) = iter.next() {
            match token {
                t @ Token::Separator(_) => return Err(Error::UnexpectedToken(t)),
                Token::Text(text) => {
                    let lower = text.to_lowercase();

                    if lower.eq(&"end") {
                        model.make_names_nice();
                        return Ok(model);
                    } else if lower.eq(&"imports") {
                        model.imports.push(Self::read_imports(&mut iter)?);
                    } else {
                        model
                            .definitions
                            .push(Self::read_definition(&mut iter, text)?);
                    }
                }
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }

    fn read_name(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        iter.next()
            .and_then(|token| {
                if let Token::Text(text) = token {
                    Some(text)
                } else {
                    None
                }
            })
            .ok_or(Error::MissingModuleName)
    }

    fn skip_after(iter: &mut IntoIter<Token>, token: &Token) -> Result<(), Error> {
        while let Some(t) = iter.next() {
            if t.eq(&token) {
                return Ok(());
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }

    fn read_imports(iter: &mut IntoIter<Token>) -> Result<Import, Error> {
        let mut imports = Import::default();
        while let Some(token) = iter.next() {
            if let Token::Text(text) = token {
                imports.what.push(text);
                match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
                    Token::Separator(s) if s == ',' => {}
                    Token::Text(s) => {
                        let lower = s.to_lowercase();
                        if s.eq(&",") {

                        } else if lower.eq(&"from") {
                            let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
                            if let Token::Text(from) = token {
                                imports.from = from;
                                Self::skip_after(iter, &Token::Separator(';'))?;
                                return Ok(imports);
                            } else {
                                return Err(Error::UnexpectedToken(token));
                            }
                        }
                    }
                    t => return Err(Error::UnexpectedToken(t)),
                }
            } else {
                return Err(Error::UnexpectedToken(token));
            }
        }
        Err(Error::UnexpectedEndOfStream)
    }

    fn read_definition(iter: &mut IntoIter<Token>, name: String) -> Result<Definition, Error> {
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, ':')?;
        Self::next_separator_ignore_case(iter, '=')?;

        let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;

        if token.text().map(|s| s.eq(&"SEQUENCE")).unwrap_or(false) {
            let token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
            match token {
                Token::Text(of) => {
                    if of.eq_ignore_ascii_case(&"OF") {
                        Ok(Definition::SequenceOf(name, Self::read_role(iter)?))
                    } else {
                        Err(Error::UnexpectedToken(Token::Text(of)))
                    }
                }
                Token::Separator(separator) => {
                    if separator == '{' {
                        let mut fields = Vec::new();

                        loop {
                            let (field, continues) = Self::read_field(iter)?;
                            fields.push(field);
                            if !continues {
                                break;
                            }
                        }

                        Ok(Definition::Sequence(name, fields))
                    } else {
                        Err(Error::UnexpectedToken(Token::Separator(separator)))
                    }
                }
            }
        } else if token.text().map(|s| s.eq(&"ENUMERATED")).unwrap_or(false) {
            Ok(Definition::Enumerated(name, Self::read_enumerated(iter)?))
        } else {
            Err(Error::UnexpectedToken(token))
        }
    }

    fn read_role(iter: &mut IntoIter<Token>) -> Result<Role, Error> {
        let text = Self::next_text(iter)?;
        if text.eq_ignore_ascii_case(&"INTEGER") {
            Self::next_separator_ignore_case(iter, '(')?;
            let start = Self::next_text(iter)?;
            Self::next_separator_ignore_case(iter, '.')?;
            Self::next_separator_ignore_case(iter, '.')?;
            let end = Self::next_text(iter)?;
            Self::next_separator_ignore_case(iter, ')')?;
            if start.eq("0") && end.eq("MAX") {
                Ok(Role::UnsignedMaxInteger)
            } else if end.eq("MAX") {
                Err(Error::UnexpectedToken(Token::Text("MAX".into())))
            } else {
                Ok(Role::Integer((
                    start.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?,
                    end.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?,
                )))
            }
        } else if text.eq_ignore_ascii_case(&"BOOLEAN") {
            Ok(Role::Boolean)
        } else if text.eq_ignore_ascii_case(&"UTF8String") {
            Ok(Role::UTF8String)
        } else {
            Ok(Role::Custom(text))
        }
    }

    fn read_enumerated(iter: &mut IntoIter<Token>) -> Result<Vec<String>, Error> {
        Self::next_separator_ignore_case(iter, '{')?;
        let mut enumeration = Vec::new();

        loop {
            enumeration.push(Self::next_text(iter)?);
            let separator = Self::next_seperator(iter)?;
            if separator == '}' {
                break;
            }
        }

        Ok(enumeration)
    }

    fn read_field(iter: &mut IntoIter<Token>) -> Result<(Field, bool), Error> {
        let mut field = Field {
            name: Self::next_text(iter)?,
            role: Self::read_role(iter)?,
            optional: false,
        };
        let mut token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        if let Some(_optional_flag) = token.text().map(|s| s.eq_ignore_ascii_case(&"OPTIONAL")) {
            field.optional = true;
            token = iter.next().ok_or(Error::UnexpectedEndOfStream)?;
        }

        let (continues, ends) = token
            .separator()
            .map(|s| (s == ',', s == '}'))
            .unwrap_or((false, false));

        if continues || ends {
            Ok((field, continues))
        } else {
            Err(Error::UnexpectedToken(token))
        }
    }

    fn next_text(iter: &mut IntoIter<Token>) -> Result<String, Error> {
        match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
            Token::Text(text) => Ok(text),
            t => Err(Error::UnexpectedToken(t)),
        }
    }

    fn next_text_ignore_case(iter: &mut IntoIter<Token>, text: &str) -> Result<(), Error> {
        let token = Self::next_text(iter)?;
        if text.eq_ignore_ascii_case(&token) {
            Ok(())
        } else {
            Err(Error::ExpectedTextGot(text.into(), token))
        }
    }

    fn next_seperator(iter: &mut IntoIter<Token>) -> Result<char, Error> {
        match iter.next().ok_or(Error::UnexpectedEndOfStream)? {
            Token::Separator(separator) => Ok(separator),
            t => Err(Error::UnexpectedToken(t)),
        }
    }

    fn next_separator_ignore_case(iter: &mut IntoIter<Token>, text: char) -> Result<(), Error> {
        let token = Self::next_seperator(iter)?;
        if token.eq_ignore_ascii_case(&text) {
            Ok(())
        } else {
            Err(Error::ExpectedSeparatorGot(text.into(), token))
        }
    }

    pub fn make_names_nice(&mut self) {
        Self::make_name_nice(&mut self.name);
        for import in self.imports.iter_mut() {
            Self::make_name_nice(&mut import.from);
        }
    }

    fn make_name_nice(name: &mut String) {
        const TO_REMOVE_AT_END: &[&'static str] = &["Module"];
        for to_remove in TO_REMOVE_AT_END.iter() {
            if name.ends_with(to_remove) {
                let new_len = name.len() - to_remove.len();
                name.truncate(new_len);
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Import {
    pub what: Vec<String>,
    pub from: String,
}

#[derive(Debug, Clone)]
pub enum Definition {
    SequenceOf(String, Role),
    Sequence(String, Vec<Field>),
    Enumerated(String, Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub role: Role,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Role {
    Boolean,
    Integer((i64, i64)),
    UnsignedMaxInteger,
    UTF8String,
    Custom(String),
}

impl Role {
    pub fn into_rust(self) -> RustType {
        RustType::from(self)
    }

    pub fn into_protobuf(self) -> ProtobufType {
        ProtobufType::from(self)
    }
}

const I8_MAX: i64 = ::std::i8::MAX as i64;
const I16_MAX: i64 = ::std::i16::MAX as i64;
const I32_MAX: i64 = ::std::i32::MAX as i64;
const I64_MAX: i64 = ::std::i64::MAX as i64;

const U8_MAX: u64 = ::std::u8::MAX as u64;
const U16_MAX: u64 = ::std::u16::MAX as u64;
const U32_MAX: u64 = ::std::u32::MAX as u64;
const U64_MAX: u64 = ::std::u64::MAX as u64;

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RustType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    String,
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl RustType {
    pub fn into_protobuf(self) -> ProtobufType {
        ProtobufType::from(self)
    }
}

impl From<Role> for RustType {
    fn from(role: Role) -> Self {
        match role {
            Role::Boolean => RustType::Bool,
            Role::Integer((lower, upper)) => {
                if lower >= 0 {
                    match upper as u64 {
                        0...U8_MAX => RustType::U8,
                        0...U16_MAX => RustType::U16,
                        0...U32_MAX => RustType::U32,
                        0...U64_MAX => RustType::U64,
                        // default is U64
                        _ => RustType::U64,
                    }
                } else {
                    let max_amplitude = lower.abs().max(upper);
                    match max_amplitude {
                        0...I8_MAX => RustType::I8,
                        0...I16_MAX => RustType::I16,
                        0...I32_MAX => RustType::I32,
                        0...I64_MAX => RustType::I64,
                        // default is I64
                        _ => RustType::I64,
                    }
                }
            }
            Role::UnsignedMaxInteger => RustType::U64,
            Role::UTF8String => RustType::String,
            Role::Custom(name) => RustType::Complex(name.clone()),
        }
    }
}

impl From<ProtobufType> for RustType {
    fn from(proto: ProtobufType) -> Self {
        match proto {
            ProtobufType::Bool => RustType::Bool,
            ProtobufType::SFixed32 => RustType::I32,
            ProtobufType::SFixed64 => RustType::I64,
            ProtobufType::UInt32 => RustType::U32,
            ProtobufType::UInt64 => RustType::U64,
            ProtobufType::SInt32 => RustType::I32,
            ProtobufType::SInt64 => RustType::I64,
            ProtobufType::String => RustType::String,
            ProtobufType::Complex(name) => RustType::Complex(name.clone()),
        }
    }
}

impl ToString for RustType {
    fn to_string(&self) -> String {
        match self {
            RustType::Bool => "bool",
            RustType::U8 => "u8",
            RustType::I8 => "i8",
            RustType::U16 => "u16",
            RustType::I16 => "i16",
            RustType::U32 => "u32",
            RustType::I32 => "i32",
            RustType::U64 => "u64",
            RustType::I64 => "i64",
            RustType::String => "String",
            RustType::Complex(name) => return name.clone(),
        }.into()
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum ProtobufType {
    Bool,
    SFixed32,
    SFixed64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    String,
    /// Indicates a complex, custom type that is
    /// not one of rusts known types
    Complex(String),
}

impl ProtobufType {
    pub fn into_rust(self) -> RustType {
        RustType::from(self)
    }
}

impl From<Role> for ProtobufType {
    fn from(role: Role) -> Self {
        match role {
            Role::Boolean => ProtobufType::Bool,
            Role::Integer((lower, upper)) => {
                if lower >= 0 {
                    match upper as u64 {
                        0...U32_MAX => ProtobufType::UInt32,
                        0...U64_MAX => ProtobufType::UInt64,
                        // default is U64
                        _ => ProtobufType::UInt64,
                    }
                } else {
                    let max_amplitude = lower.abs().max(upper);
                    match max_amplitude {
                        0...I32_MAX => ProtobufType::SInt32,
                        0...I64_MAX => ProtobufType::SInt32,
                        // default is I64
                        _ => ProtobufType::SInt64,
                    }
                }
            }
            Role::UnsignedMaxInteger => ProtobufType::UInt64,
            Role::Custom(name) => ProtobufType::Complex(name.clone()),
            Role::UTF8String => ProtobufType::String,
        }
    }
}

impl From<RustType> for ProtobufType {
    fn from(rust: RustType) -> Self {
        match rust {
            RustType::Bool => ProtobufType::Bool,
            RustType::U8 => ProtobufType::UInt32,
            RustType::I8 => ProtobufType::SInt32,
            RustType::U16 => ProtobufType::UInt32,
            RustType::I16 => ProtobufType::SInt32,
            RustType::U32 => ProtobufType::UInt32,
            RustType::I32 => ProtobufType::SInt32,
            RustType::U64 => ProtobufType::UInt64,
            RustType::I64 => ProtobufType::SInt64,
            RustType::String => ProtobufType::String,
            RustType::Complex(name) => ProtobufType::Complex(name.clone()),
        }
    }
}

impl ToString for ProtobufType {
    fn to_string(&self) -> String {
        match self {
            ProtobufType::Bool => "bool",
            ProtobufType::SFixed32 => "sfixed32",
            ProtobufType::SFixed64 => "sfixed64",
            ProtobufType::UInt32 => "uint32",
            ProtobufType::UInt64 => "uint64",
            ProtobufType::SInt32 => "sint32",
            ProtobufType::SInt64 => "sint64",
            ProtobufType::String => "string",
            ProtobufType::Complex(name) => return name.clone(),
        }.into()
    }
}
