use crate::gen::protobuf::Error as ProtobufGeneratorError;
use crate::gen::protobuf::ProtobufDefGenerator as ProtobufGenerator;
use crate::gen::rust::RustCodeGenerator as RustGenerator;
use crate::gen::sql::Error as SqlGeneratorError;
use crate::gen::sql::SqlDefGenerator as SqlGenerator;
use crate::gen::Generator;
use crate::model::protobuf::ToProtobufModel;
use crate::model::sql::ToSqlModel;
use crate::model::Error as ModelError;
use crate::model::Model;
use crate::parser::Error as ParserError;
use crate::parser::Parser;
use std::io::Error as IoError;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    RustGenerator,
    ProtobufGenerator(ProtobufGeneratorError),
    SqlGenerator(SqlGeneratorError),
    Model(ModelError),
    Parser(ParserError),
    Io(IoError),
}

impl From<ProtobufGeneratorError> for Error {
    fn from(g: ProtobufGeneratorError) -> Self {
        Error::ProtobufGenerator(g)
    }
}

impl From<SqlGeneratorError> for Error {
    fn from(e: SqlGeneratorError) -> Self {
        Error::SqlGenerator(e)
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

pub fn convert_to_rust<F: AsRef<Path>, D: AsRef<Path>, A: FnOnce(&mut RustGenerator)>(
    file: F,
    dir: D,
    custom_adjustments: A,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::default().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = RustGenerator::default();
    generator.add_model(model.to_rust());

    custom_adjustments(&mut generator);

    let output = generator.to_string().map_err(|_| Error::RustGenerator)?;

    let mut files = Vec::new();
    for (file, content) in output {
        ::std::fs::write(dir.as_ref().join(&file), content)?;
        files.push(file);
    }
    Ok(files)
}

pub fn convert_to_proto<F: AsRef<Path>, D: AsRef<Path>>(
    file: F,
    dir: D,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::default().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = ProtobufGenerator::default();
    generator.add_model(model.to_rust().to_protobuf());
    let output = generator.to_string()?;

    let mut files = Vec::new();
    for (file, content) in output {
        ::std::fs::write(dir.as_ref().join(&file), content)?;
        files.push(file);
    }
    Ok(files)
}

pub fn convert_to_sql<F: AsRef<Path>, D: AsRef<Path>>(
    file: F,
    dir: D,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Parser::default().parse(&input)?;
    let model = Model::try_from(tokens)?;
    let mut generator = SqlGenerator::default();
    generator.add_model(model.to_rust().to_sql());
    let output = generator.to_string()?;

    let mut files = Vec::new();
    for (file, content) in output {
        ::std::fs::write(dir.as_ref().join(&file), content)?;
        files.push(file);
    }
    Ok(files)
}
