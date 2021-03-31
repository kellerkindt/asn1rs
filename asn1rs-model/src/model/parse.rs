use std::iter::Peekable;

use crate::model::err::ErrorKind;
use crate::parser::Token;

pub trait PeekableTokens {
    fn peek_or_err(&mut self) -> Result<&Token, ErrorKind>;

    fn peek_is_text_eq(&mut self, text: &str) -> bool;

    fn peek_is_text_eq_ignore_case(&mut self, text: &str) -> bool;

    fn peek_is_separator_eq(&mut self, separator: char) -> bool;

    #[inline]
    fn peek_is_text_and_satisfies<F: FnOnce(&str) -> bool>(&mut self, probe: F) -> bool {
        self.peek_or_err()
            .ok()
            .and_then(Token::text)
            .map(|t| probe(t))
            .unwrap_or(false)
    }

    fn next_or_err(&mut self) -> Result<Token, ErrorKind>;

    fn next_text_or_err(&mut self) -> Result<String, ErrorKind>;

    fn next_text_eq_ignore_case_or_err(&mut self, text: &str) -> Result<Token, ErrorKind>;

    fn next_text_eq_any_ignore_case_or_err(&mut self, texts: &[&str]) -> Result<Token, ErrorKind>;

    #[inline]
    fn next_is_text_and_eq_ignore_case(&mut self, text: &str) -> bool {
        self.next_text_eq_ignore_case_or_err(text).is_ok()
    }

    fn next_if_separator_and_eq(&mut self, separator: char) -> Result<Token, ErrorKind>;

    #[inline]
    fn next_separator_eq_or_err(&mut self, separator: char) -> Result<(), ErrorKind> {
        self.next_if_separator_and_eq(separator).map(drop)
    }

    #[inline]
    fn next_is_separator_and_eq(&mut self, separator: char) -> bool {
        self.next_separator_eq_or_err(separator).is_ok()
    }
}

impl<T: Iterator<Item = Token>> PeekableTokens for Peekable<T> {
    #[inline]
    fn peek_or_err(&mut self) -> Result<&Token, ErrorKind> {
        self.peek().ok_or(ErrorKind::UnexpectedEndOfStream)
    }

    #[inline]
    fn peek_is_text_eq(&mut self, text: &str) -> bool {
        self.peek()
            .and_then(Token::text)
            .map(|t| t.eq(text))
            .unwrap_or(false)
    }

    #[inline]
    fn peek_is_text_eq_ignore_case(&mut self, text: &str) -> bool {
        self.peek()
            .and_then(Token::text)
            .map(|t| text.eq_ignore_ascii_case(t))
            .unwrap_or(false)
    }

    #[inline]
    fn peek_is_separator_eq(&mut self, separator: char) -> bool {
        self.peek()
            .map(|t| t.eq_separator(separator))
            .unwrap_or(false)
    }

    #[inline]
    fn next_or_err(&mut self) -> Result<Token, ErrorKind> {
        self.next().ok_or(ErrorKind::UnexpectedEndOfStream)
    }

    #[inline]
    fn next_text_or_err(&mut self) -> Result<String, ErrorKind> {
        let peeked = self.peek_or_err()?;
        if peeked.text().is_some() {
            let token = self.next_or_err()?;
            debug_assert!(token.text().is_some());
            match token {
                Token::Separator(..) => unreachable!(),
                Token::Text(_, text) => Ok(text),
            }
        } else {
            Err(ErrorKind::ExpectedText(peeked.clone()))
        }
    }

    #[inline]
    fn next_text_eq_ignore_case_or_err(&mut self, text: &str) -> Result<Token, ErrorKind> {
        let peeked = self.peek_or_err()?;
        if peeked.eq_text_ignore_ascii_case(text) {
            let token = self.next_or_err()?;
            debug_assert!(token.eq_text_ignore_ascii_case(text));
            Ok(token)
        } else {
            Err(ErrorKind::ExpectedTextGot(text.to_string(), peeked.clone()))
        }
    }

    #[inline]
    fn next_text_eq_any_ignore_case_or_err(&mut self, texts: &[&str]) -> Result<Token, ErrorKind> {
        let peeked = self.peek_or_err()?;
        if matches!(peeked, Token::Text(_, token) if texts.iter().any(|text| token.eq_ignore_ascii_case(text)))
        {
            let token = self.next_or_err()?;
            debug_assert!(
                matches!(&token, Token::Text(_, token) if texts.iter().any(|text| token.eq_ignore_ascii_case(text)))
            );
            Ok(token)
        } else {
            Err(ErrorKind::UnexpectedToken(peeked.clone()))
        }
    }

    #[inline]
    fn next_if_separator_and_eq(&mut self, separator: char) -> Result<Token, ErrorKind> {
        let peeked = self.peek_or_err()?;
        if peeked.eq_separator(separator) {
            let token = self.next_or_err()?;
            debug_assert!(token.eq_separator(separator));
            Ok(token)
        } else {
            Err(ErrorKind::ExpectedSeparatorGot(separator, peeked.clone()))
        }
    }
}
