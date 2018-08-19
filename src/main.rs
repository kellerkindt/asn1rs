extern crate codegen;

mod gen;
mod model;
mod parser;

use std::fs::File;
use std::io::Error as IoError;

use gen::Error as GeneratorError;
use gen::Generator;

use model::Error as ModelError;
use model::Model;

use parser::Error as ParserError;
use parser::Parser;
use parser::Token;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Generator(GeneratorError),
    Model(ModelError),
    Parser(ParserError),
    Io(IoError),
}

impl From<GeneratorError> for Error {
    fn from(g: GeneratorError) -> Self {
        Error::Generator(g)
    }
}

impl From<ModelError> for Error {
    fn from(m: ModelError) -> Self {
        Error::Model(m)
    }
}

impl From<ParserError> for Error {
    fn from(p: ParserError) -> Self {
        Error::Parser(p)
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

const EXAMPLE: &'static str = include_str!("../../ref/def.asn1");

fn main() {
    convert("/home/mi7wa6/mec-view/svn-sources/trunk/MECViewServerSDK/proto/general.asn1", "../asn1_uper/src/asn1/").unwrap();
    convert("/home/mi7wa6/mec-view/svn-sources/trunk/MECViewServerSDK/proto/environment.asn1", "../asn1_uper/src/asn1/").unwrap();
    /*
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
    }*/
}

fn convert<F: AsRef<Path>, D: AsRef<Path>>(file: F, dir: D) -> Result<(), Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::new().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = Generator::new();
    generator.add_model(model);
    let output = generator.to_string()?;

    let dir = dir.as_ref().clone();
    for (file, content) in output {
        ::std::fs::write(dir.join(file), content)?;
    }
    Ok(())
}
