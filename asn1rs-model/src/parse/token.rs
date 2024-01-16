use crate::parse::Location;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone)]
pub enum Token {
    Text(Location, String),
    Separator(Location, char),
}

impl From<char> for Token {
    fn from(separator: char) -> Self {
        Token::Separator(Location::default(), separator)
    }
}

impl From<String> for Token {
    fn from(text: String) -> Self {
        Token::Text(Location::default(), text)
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Token::Text(_, text) => write!(f, "\"{}\"", text),
            Token::Separator(_, separator) => write!(f, "\'{}\'", separator),
        }
    }
}

impl Token {
    pub fn append(self, other: Token) -> (Token, Option<Token>) {
        match (self, other) {
            (Token::Text(location, mut text), Token::Text(_, other)) => (
                Token::Text(location, {
                    text.push_str(&other);
                    text
                }),
                None,
            ),
            (a, b) => (a, Some(b)),
        }
    }

    pub fn location(&self) -> Location {
        match self {
            Token::Text(location, _) => *location,
            Token::Separator(location, _) => *location,
        }
    }

    pub fn eq_text(&self, text: &str) -> bool {
        self.text().map(|t| t.eq(text)).unwrap_or(false)
    }

    pub fn eq_text_ignore_ascii_case(&self, text: &str) -> bool {
        self.text()
            .map(|t| t.eq_ignore_ascii_case(text))
            .unwrap_or(false)
    }

    pub fn eq_separator(&self, separator: char) -> bool {
        self.separator().map(|s| s == separator).unwrap_or(false)
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Token::Text(_, text) => Some(text),
            _ => None,
        }
    }

    pub fn separator(&self) -> Option<char> {
        match self {
            Token::Separator(_, char) => Some(*char),
            _ => None,
        }
    }

    pub fn is_text(&self) -> bool {
        self.text().is_some()
    }

    pub fn is_separator(&self) -> bool {
        self.separator().is_some()
    }

    pub fn into_text(self) -> Option<String> {
        if let Token::Text(_, text) = self {
            Some(text)
        } else {
            None
        }
    }

    pub fn into_text_or_else<E, F: FnOnce(Token) -> E>(self, f: F) -> Result<String, E> {
        match self {
            Token::Text(_, text) => Ok(text),
            token => Err(f(token)),
        }
    }

    pub fn into_separator_or_else<E, F: FnOnce(Token) -> E>(self, f: F) -> Result<char, E> {
        match self {
            Token::Separator(_, separator) => Ok(separator),
            token => Err(f(token)),
        }
    }
}
