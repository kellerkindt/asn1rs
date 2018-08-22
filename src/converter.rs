use gen::protobuf::Error as ProtobufGeneratorError;
use gen::protobuf::Generator as ProtobufGenerator;
use gen::rust::Error as RustGeneratorError;
use gen::rust::Generator as RustGenerator;
use model::Error as ModelError;
use model::Model;
use parser::Error as ParserError;
use parser::Parser;

use std::io::Error as IoError;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    RustGenerator(RustGeneratorError),
    ProtobufGenerator(ProtobufGeneratorError),
    Model(ModelError),
    Parser(ParserError),
    Io(IoError),
}

impl From<RustGeneratorError> for Error {
    fn from(g: RustGeneratorError) -> Self {
        Error::RustGenerator(g)
    }
}

impl From<ProtobufGeneratorError> for Error {
    fn from(g: ProtobufGeneratorError) -> Self {
        Error::ProtobufGenerator(g)
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

pub fn convert_to_rust<F: AsRef<Path>, D: AsRef<Path>>(
    file: F,
    dir: D,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::new().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = RustGenerator::new();
    generator.add_model(model);
    let output = generator.to_string()?;

    let dir = dir.as_ref().clone();
    let mut files = Vec::new();
    for (file, content) in output {
        ::std::fs::write(dir.join(&file), content)?;
        files.push(file);
    }
    Ok(files)
}

pub fn convert_to_proto<F: AsRef<Path>, D: AsRef<Path>>(
    file: F,
    dir: D,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::new().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = ProtobufGenerator::default();
    generator.add_model(model);
    let output = generator.generate()?;

    let dir = dir.as_ref().clone();
    let mut files = Vec::new();
    for (file, content) in output {
        ::std::fs::write(dir.join(&file), content)?;
        files.push(file);
    }
    Ok(files)
}
