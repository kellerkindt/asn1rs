use crate::gen::protobuf::Error as ProtobufGeneratorError;
use crate::gen::protobuf::ProtobufDefGenerator as ProtobufGenerator;
use crate::gen::rust::RustCodeGenerator as RustGenerator;
use crate::gen::Generator;
use crate::model::lor::Error as ResolveError;
use crate::model::protobuf::ToProtobufModel;
use crate::model::Model;
use crate::model::{Error as ModelError, MultiModuleResolver};
use crate::parser::Tokenizer;
use std::collections::HashMap;
use std::io::Error as IoError;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    RustGenerator,
    ProtobufGenerator(ProtobufGeneratorError),
    Model(ModelError),
    Io(IoError),
    ResolveError(ResolveError),
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

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

impl From<ResolveError> for Error {
    fn from(e: ResolveError) -> Self {
        Error::ResolveError(e)
    }
}

#[derive(Default)]
pub struct Converter {
    models: MultiModuleResolver,
}

impl Converter {
    pub fn load_file<F: AsRef<Path>>(&mut self, file: F) -> Result<(), Error> {
        let input = ::std::fs::read_to_string(file)?;
        let tokens = Tokenizer.parse(&input);
        let model = Model::try_from(tokens)?;
        self.models.push(model);
        Ok(())
    }

    pub fn to_rust<D: AsRef<Path>, A: Fn(&mut RustGenerator)>(
        &self,
        directory: D,
        custom_adjustments: A,
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        let models = self.models.try_resolve_all()?;
        let scope = models.iter().collect::<Vec<_>>();
        let mut files = HashMap::with_capacity(models.len());

        for model in &models {
            let mut generator = RustGenerator::default();
            generator.add_model(model.to_rust_with_scope(&scope[..]));

            custom_adjustments(&mut generator);

            files.insert(
                model.name.clone(),
                generator
                    .to_string()
                    .map_err(|_| Error::RustGenerator)?
                    .into_iter()
                    .map(|(file, content)| {
                        ::std::fs::write(directory.as_ref().join(&file), content)?;
                        Ok::<_, Error>(file)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }

        Ok(files)
    }

    pub fn to_protobuf<D: AsRef<Path>>(
        &self,
        directory: D,
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        let models = self.models.try_resolve_all()?;
        let scope = models.iter().collect::<Vec<_>>();
        let mut files = HashMap::with_capacity(models.len());

        for model in &models {
            let mut generator = ProtobufGenerator::default();
            generator.add_model(model.to_rust_with_scope(&scope[..]).to_protobuf());

            files.insert(
                model.name.clone(),
                generator
                    .to_string()?
                    .into_iter()
                    .map(|(file, content)| {
                        ::std::fs::write(directory.as_ref().join(&file), content)?;
                        Ok::<_, Error>(file)
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }

        Ok(files)
    }
}

#[deprecated(note = "Use the Converter instead")]
pub fn convert_to_rust<F: AsRef<Path>, D: AsRef<Path>, A: Fn(&mut RustGenerator)>(
    file: F,
    dir: D,
    custom_adjustments: A,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Tokenizer.parse(&input);
    let model = Model::try_from(tokens)?.try_resolve()?;
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

#[deprecated(note = "Use the Converter instead")]
pub fn convert_to_proto<F: AsRef<Path>, D: AsRef<Path>>(
    file: F,
    dir: D,
) -> Result<Vec<String>, Error> {
    let input = ::std::fs::read_to_string(file)?;
    let tokens = Tokenizer.parse(&input);
    let model = Model::try_from(tokens)?.try_resolve()?;
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
