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
            Ok(Role::Integer((
                start.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?,
                if end.eq_ignore_ascii_case(&"MAX") {
                    ::std::i64::MAX
                } else {
                    end.parse::<i64>().map_err(|_| Error::InvalidRangeValue)?
                },
            )))
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

#[derive(Debug, Clone)]
pub enum Role {
    Boolean,
    Integer((i64, i64)),
    UTF8String,
    Custom(String),
}
