mod error;
mod location;
mod token;
mod tokenizer;

pub use error::Error;
pub use error::ErrorKind;
pub use location::Location;
pub use token::Token;
pub use tokenizer::Tokenizer;

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
