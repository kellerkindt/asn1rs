use crate::generators::rust::RustCodeGenerator as RustGenerator;
use crate::generators::Generator;
use crate::parser::Tokenizer;
use asn1rs_model::asn::MultiModuleResolver;
use asn1rs_model::model::Model;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    RustGenerator,
    #[cfg(feature = "protobuf")]
    ProtobufGenerator(crate::generators::protobuf::Error),
    Model(asn1rs_model::model::err::Error),
    Io(std::io::Error),
    ResolveError(asn1rs_model::model::lit_or_ref::Error),
}

#[cfg(feature = "protobuf")]
impl From<crate::generators::protobuf::Error> for Error {
    fn from(g: crate::generators::protobuf::Error) -> Self {
        Error::ProtobufGenerator(g)
    }
}

impl From<asn1rs_model::model::err::Error> for Error {
    fn from(m: asn1rs_model::model::err::Error) -> Self {
        Error::Model(m)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<asn1rs_model::model::lit_or_ref::Error> for Error {
    fn from(e: asn1rs_model::model::lit_or_ref::Error) -> Self {
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

    #[cfg(feature = "protobuf")]
    pub fn to_protobuf<D: AsRef<Path>>(
        &self,
        directory: D,
    ) -> Result<HashMap<String, Vec<String>>, Error> {
        use asn1rs_model::model::protobuf::ToProtobufModel;

        let models = self.models.try_resolve_all()?;
        let scope = models.iter().collect::<Vec<_>>();
        let mut files = HashMap::with_capacity(models.len());

        for model in &models {
            let mut generator = crate::generators::protobuf::ProtobufDefGenerator::default();
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
