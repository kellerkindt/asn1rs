mod model;
mod parser;

use model::Error as ModelError;
use model::Model;

use parser::Error as ParserError;
use parser::Parser;
use parser::Token;

const EXAMPLE: &'static str = include_str!("environment.asn1");

fn main() {
    println!("Hello, world!");
    let parser = Parser::new();
    //let tokens = parser.parse("HEADER ::= SEQUENCE { header INTEGER (-100..20) OPTIONAL }").unwrap();
    let tokens = parser.parse(EXAMPLE).unwrap();
    println!("Tokens: {:?}", tokens);
    let model = Model::try_from(tokens).unwrap();
    println!("{:#?}", model);
}
