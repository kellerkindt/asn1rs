use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq, Eq)]
pub struct Location {
    line: usize,
    column: usize,
}

impl Location {
    pub const fn at(line: usize, column: usize) -> Location {
        Self { line, column }
    }

    pub const fn line(&self) -> usize {
        self.line
    }

    pub const fn column(&self) -> usize {
        self.column
    }
}

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
    fn append(self, other: Token) -> (Token, Option<Token>) {
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

#[derive(Default)]
pub struct Tokenizer;

impl Tokenizer {
    /// Tokenize the given ASN.1 string.
    /// Parse the string line by line and character by character.
    /// Exclude comments as defined in 12.6.2-4  ITU-T Rec. X.680 (02/2021)
    /// Ignore single-line comments defined with "--".
    /// Ignore multi-line comments defined with /*  */.
    /// Comment terminates when a matching "*/" has been found for each "/*"
    pub fn parse(&self, asn: &str) -> Vec<Token> {
        let mut previous = None;
        let mut tokens = Vec::new();
        let mut nest_lvl = 0; // Nest level of the comments

        for (line_0, line) in asn.lines().enumerate() {
            let mut token = None;
            let mut content_iterator = line.chars().enumerate().peekable();

            while let Some((column_0, char)) = content_iterator.next() {
                if nest_lvl > 0 {
                    match char {
                        '*' => {
                            if let Some((_, '/')) = content_iterator.peek() {
                                nest_lvl -= 1;
                                content_iterator.next(); // remove closing '/'
                            }
                        }
                        '/' => {
                            if let Some((_, '*')) = content_iterator.peek() {
                                nest_lvl += 1;
                                content_iterator.next(); // remove opening '*'
                            }
                        }
                        _ => {
                            if content_iterator.peek().is_none()
                                && line_0 == asn.lines().count() - 1
                            {
                                panic!("The file has unclosed comment blocks. Nested comment blocks are counted.");
                            } else {
                                continue;
                            }
                        }
                    }
                    continue;
                }
                // Get rid of one-line comments. Can also happen immediately after closing block comment
                if nest_lvl == 0
                    && char == '-'
                    && content_iterator.peek().map(|&(_, ch)| ch) == Some('-')
                {
                    content_iterator.next(); // remove second '-'
                    break; // ignore rest of the line
                }
                match char {
                    '/' if content_iterator.peek().map(|&(_, ch)| ch) == Some('*') => {
                        content_iterator.next(); // remove opening '*'
                        nest_lvl += 1;
                    }
                    // asn syntax
                    ':' | ';' | '=' | '(' | ')' | '{' | '}' | '.' | ',' | '[' | ']' | '\''
                    | '"' => {
                        token = Some(Token::Separator(
                            Location::at(line_0 + 1, column_0 + 1),
                            char,
                        ))
                    }
                    // text
                    c if !c.is_control() && c != ' ' => {
                        token = Some(Token::Text(
                            Location::at(line_0 + 1, column_0 + 1),
                            format!("{}", c),
                        ));
                    }
                    // text separator
                    ' ' | '\r' | '\n' | '\t' => {
                        if let Some(token) = previous.take() {
                            tokens.push(token);
                        }
                    }
                    c => eprintln!(
                        "Ignoring unexpected character: {}-0x{:02x}-{:03}",
                        c, c as u8, c as u8
                    ),
                }

                if let Some(token) = token.take() {
                    previous = match previous {
                        None => Some(token),
                        Some(current) => {
                            let (token, second) = current.append(token);
                            match second {
                                None => Some(token),
                                Some(next) => {
                                    tokens.push(token);
                                    Some(next)
                                }
                            }
                        }
                    }
                }
            }

            if let Some(token) = previous.take() {
                tokens.push(token);
            }
        }

        if let Some(token) = previous {
            tokens.push(token);
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_separator_tokens_not_merged() {
        let result = Tokenizer.parse(":;=(){}.,[]");
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator(';'));
        assert!(iter.next().unwrap().eq_separator('='));
        assert!(iter.next().unwrap().eq_separator('('));
        assert!(iter.next().unwrap().eq_separator(')'));
        assert!(iter.next().unwrap().eq_separator('{'));
        assert!(iter.next().unwrap().eq_separator('}'));
        assert!(iter.next().unwrap().eq_separator('.'));
        assert!(iter.next().unwrap().eq_separator(','));
        assert!(iter.next().unwrap().eq_separator('['));
        assert!(iter.next().unwrap().eq_separator(']'));
        assert!(iter.next().is_none());
    }

    #[test]
    pub fn test_text_between_seapators_is_represented_as_one_text_token() {
        let result = Tokenizer.parse("::=ASN{");
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator('='));
        assert!(iter.next().unwrap().eq_text("ASN"));
        assert!(iter.next().unwrap().eq_separator('{'));
        assert!(iter.next().is_none());
    }

    #[test]
    pub fn test_invisible_separator_characters() {
        let result = Tokenizer.parse("a b\rc\nd\te AB\rCD\nEF\tGH aa  bb\r\rcc\n\ndd\t\tee");
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_text("a"));
        assert!(iter.next().unwrap().eq_text("b"));
        assert!(iter.next().unwrap().eq_text("c"));
        assert!(iter.next().unwrap().eq_text("d"));
        assert!(iter.next().unwrap().eq_text("e"));
        assert!(iter.next().unwrap().eq_text("AB"));
        assert!(iter.next().unwrap().eq_text("CD"));
        assert!(iter.next().unwrap().eq_text("EF"));
        assert!(iter.next().unwrap().eq_text("GH"));
        assert!(iter.next().unwrap().eq_text("aa"));
        assert!(iter.next().unwrap().eq_text("bb"));
        assert!(iter.next().unwrap().eq_text("cc"));
        assert!(iter.next().unwrap().eq_text("dd"));
        assert!(iter.next().unwrap().eq_text("ee"));
        assert!(iter.next().is_none());
    }

    #[test]
    pub fn test_token_text() {
        let token = Token::from("some text".to_string());
        assert_eq!(token.text(), Some("some text"));
        assert_eq!(token.separator(), None);
    }

    #[test]
    pub fn test_token_separator() {
        let result = Tokenizer.parse("AS\x00N");
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_text("ASN"));
        assert!(iter.next().is_none());
    }

    #[test]
    pub fn test_control_char_is_ignored() {
        let token = Token::from(':');
        assert_eq!(token.text(), None);
        assert_eq!(token.separator(), Some(':'),)
    }

    #[test]
    pub fn test_ignores_line_comments() {
        let result = Tokenizer::default().parse(
            r"
                Some ::= None -- very clever
                        -- ignore true ::= false
        ",
        );
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_text("Some"));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator('='));
        assert!(iter.next().unwrap().eq_text("None"));
        assert!(iter.next().is_none());
    }
    #[test]
    pub fn test_ignores_multiline_comments() {
        let result = Tokenizer::default().parse(
            r"
            ASN1 DEFINITION ::= BEGIN
            /* This is a comment */
            -- This is also a comment
            SomeTypeDef ::= SEQUENCE {
            /* Nested comment level 1
               /* Nested comment -- level 2 */
            still in level 1 comment */
            integer INTEGER
            }
            END",
        );
        let mut iter = result.into_iter();
        assert!(iter.next().unwrap().eq_text("ASN1"));
        assert!(iter.next().unwrap().eq_text("DEFINITION"));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator('='));
        assert!(iter.next().unwrap().eq_text("BEGIN"));
        assert!(iter.next().unwrap().eq_text("SomeTypeDef"));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator(':'));
        assert!(iter.next().unwrap().eq_separator('='));
        assert!(iter.next().unwrap().eq_text("SEQUENCE"));
        assert!(iter.next().unwrap().eq_separator('{'));
        assert!(iter.next().unwrap().eq_text("integer"));
        assert!(iter.next().unwrap().eq_text("INTEGER"));
        assert!(iter.next().unwrap().eq_separator('}'));
        assert!(iter.next().unwrap().eq_text("END"));
        assert!(iter.next().is_none());
    }

    #[test]
    #[should_panic(
        expected = "The file has unclosed comment blocks. Nested comment blocks are counted."
    )]
    pub fn test_unclosed_comment() {
        let _ = Tokenizer::default().parse(
            r"
            ASN1 DEFINITION ::= BEGIN
            /* This is a comment
            SomeTypeDef ::= SEQUENCE {
            /* Nested comment level 1
               /* Nested comment -- level 2 */
            still in level 1 comment */
            integer INTEGER
            }
            END",
        );
    }

    #[test]
    pub fn test_token_is_separator() {
        assert!(Token::Separator(Location::default(), ',').is_separator());
    }

    #[test]
    pub fn test_token_is_text() {
        assert!(Token::Text(Location::default(), String::default()).is_text());
    }

    #[test]
    pub fn test_token_location_separator() {
        let location = Location::at(42, 1337);
        assert_eq!(location, Token::Separator(location, ',').location());
    }

    #[test]
    pub fn test_token_location_text() {
        let location = Location::at(42, 1337);
        assert_eq!(
            location,
            Token::Text(location, String::default()).location()
        );
    }

    #[test]
    pub fn test_token_eq_text() {
        assert!(Token::Text(Location::default(), "aBc".to_string()).eq_text("aBc"));
        assert!(!Token::Text(Location::default(), "aBc".to_string()).eq_text("abc"));
        assert!(!Token::Text(Location::default(), "aBc".to_string()).eq_text("cde"));
    }

    #[test]
    pub fn test_token_eq_text_ignore_ascii_case() {
        assert!(
            Token::Text(Location::default(), "aBc".to_string()).eq_text_ignore_ascii_case("aBc")
        );
        assert!(
            Token::Text(Location::default(), "aBc".to_string()).eq_text_ignore_ascii_case("abc")
        );
        assert!(
            !Token::Text(Location::default(), "aBc".to_string()).eq_text_ignore_ascii_case("cde")
        );
    }

    #[test]
    pub fn test_token_display_text() {
        assert_eq!(
            "\"The text\"",
            format!(
                "{}",
                Token::Text(Location::default(), "The text".to_string())
            )
        );
    }

    #[test]
    pub fn test_token_display_separator() {
        assert_eq!(
            "'.'",
            format!("{}", Token::Separator(Location::default(), '.'))
        );
    }

    #[test]
    pub fn test_token_into_text_none() {
        assert_eq!(None, Token::Separator(Location::default(), '.').into_text());
    }

    #[test]
    pub fn test_token_into_text_or_else_succeed() {
        assert_eq!(
            Ok("SEQUENCE".to_string()),
            Token::Text(Location::default(), "SEQUENCE".to_string())
                .into_text_or_else(|_| unreachable!())
        );
    }

    #[test]
    pub fn test_token_into_text_or_else_fail() {
        assert_eq!(
            Err(()),
            Token::Separator(Location::default(), '.').into_text_or_else(|_| ())
        );
    }

    #[test]
    pub fn test_token_into_separator_or_else_succeed() {
        assert_eq!(
            Ok('.'),
            Token::Separator(Location::default(), '.').into_separator_or_else(|_| unreachable!())
        );
    }

    #[test]
    pub fn test_token_into_separator_or_else_fail() {
        assert_eq!(
            Err(()),
            Token::Text(Location::default(), String::default()).into_separator_or_else(|_| ())
        );
    }
}
