#[derive(Debug, PartialOrd, PartialEq)]
pub enum Token {
    Text(String),
    Separator(char),
}

impl Token {
    fn append(self, other: Token) -> (Token, Option<Token>) {
        match (self, other) {
            (Token::Text(mut text), Token::Text(other)) => (
                Token::Text({
                    text.push_str(&other);
                    text
                }),
                None,
            ),
            (a, b) => (a, Some(b)),
        }
    }

    pub fn text(&self) -> Option<&String> {
        match self {
            Token::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn separator(&self) -> Option<char> {
        match self {
            Token::Separator(char) => Some(*char),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Tokenizer;

impl Tokenizer {
    pub fn parse(&self, asn: &str) -> Vec<Token> {
        let mut previous = None;
        let mut tokens = Vec::new();

        for line in asn.lines() {
            let mut token = None;
            let content = line.split("--").next(); // get rid of one-line comments

            for char in content.iter().map(|c| c.chars()).flatten() {
                match char {
                    // asn syntax
                    ':' | ';' | '=' | '(' | ')' | '{' | '}' | '.' | ',' => {
                        token = Some(Token::Separator(char))
                    }
                    // text
                    c if !c.is_control() && c != ' ' => {
                        token = Some(Token::Text(format!("{}", c)));
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
        let result = Tokenizer.parse(":;=(){}.,");
        assert_eq!(
            result,
            vec![
                Token::Separator(':'),
                Token::Separator(';'),
                Token::Separator('='),
                Token::Separator('('),
                Token::Separator(')'),
                Token::Separator('{'),
                Token::Separator('}'),
                Token::Separator('.'),
                Token::Separator(','),
            ]
        )
    }

    #[test]
    pub fn test_text_between_seapators_is_represented_as_one_text_token() {
        let result = Tokenizer.parse("::=ASN{");
        assert_eq!(
            result,
            vec![
                Token::Separator(':'),
                Token::Separator(':'),
                Token::Separator('='),
                Token::Text("ASN".to_string()),
                Token::Separator('{'),
            ]
        )
    }

    #[test]
    pub fn test_invisible_separator_characters() {
        let result = Tokenizer.parse("a b\rc\nd\te AB\rCD\nEF\tGH aa  bb\r\rcc\n\ndd\t\tee");
        assert_eq!(
            result,
            vec![
                Token::Text("a".to_string()),
                Token::Text("b".to_string()),
                Token::Text("c".to_string()),
                Token::Text("d".to_string()),
                Token::Text("e".to_string()),
                Token::Text("AB".to_string()),
                Token::Text("CD".to_string()),
                Token::Text("EF".to_string()),
                Token::Text("GH".to_string()),
                Token::Text("aa".to_string()),
                Token::Text("bb".to_string()),
                Token::Text("cc".to_string()),
                Token::Text("dd".to_string()),
                Token::Text("ee".to_string()),
            ]
        )
    }

    #[test]
    pub fn test_token_text() {
        let token = Token::Text("some text".to_string());
        assert_eq!(token.text(), Some(&"some text".to_string()));
        assert_eq!(token.separator(), None);
    }

    #[test]
    pub fn test_token_separator() {
        let result = Tokenizer.parse("AS\x00N");
        assert_eq!(result, vec![Token::Text("ASN".to_string())])
    }

    #[test]
    pub fn test_control_char_is_ignored() {
        let token = Token::Separator(':');
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
        assert_eq!(Token::Text("Some".to_string()), iter.next().unwrap());
        assert_eq!(Token::Separator(':'), iter.next().unwrap());
        assert_eq!(Token::Separator(':'), iter.next().unwrap());
        assert_eq!(Token::Separator('='), iter.next().unwrap());
        assert_eq!(Token::Text("None".to_string()), iter.next().unwrap());
        assert_eq!(None, iter.next());
    }
}
