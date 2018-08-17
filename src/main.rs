extern crate codegen;

mod gen;
mod model;
mod parser;

use gen::Generator;

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

    let mut generator = Generator::new();
    generator.add_model(model);
    for (filePath, fileContent) in generator.to_string().unwrap().iter() {
        println!("### {}:", filePath);
        println!("{}", fileContent);
    }
}
