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
            Token::Text(text) => Some(&text),
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

#[derive(Debug)]
pub enum Error {}

pub struct Parser;

impl Parser {
    pub fn new() -> Parser {
        Parser
    }

    pub fn parse(&self, asn: &str) -> Result<Vec<Token>, Error> {
        let mut iter = asn.chars();
        let mut token = None;
        let mut previous = None;
        let mut tokens = Vec::new();

        while let Some(char) = iter.next() {
            token = None;
            match char {
                ':' | ';' | '=' | '(' | ')' | '{' | '}' | '.' | ',' => {
                    token = Some(Token::Separator(char))
                }
                c if !c.is_control() && c != ' ' => {
                    token = Some(Token::Text(format!("{}", c)));
                }
                ' ' | '\r' | '\n' | '\t' => if let Some(token) = previous.take() {
                    tokens.push(token);
                },
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

        if let Some(token) = token {
            tokens.push(token);
        }
        Ok(tokens)
    }
}
