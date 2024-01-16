use crate::parse::{Location, Token};

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
