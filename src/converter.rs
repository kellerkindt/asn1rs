use gen::rust::Error as GeneratorError;
use gen::rust::Generator;
use model::Error as ModelError;
use model::Model;
use parser::Error as ParserError;
use parser::Parser;

use std::io::Error as IoError;
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

pub fn convert<F: AsRef<Path>, D: AsRef<Path>>(file: F, dir: D) -> Result<(), Error> {
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
