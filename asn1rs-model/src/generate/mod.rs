#[cfg(feature = "protobuf")]
pub mod protobuf;
pub mod rust;
pub mod walker;

pub use self::rust::RustCodeGenerator;

use crate::model::{Model, Target};

pub trait Generator<T: Target> {
    type Error;

    fn add_model(&mut self, model: Model<T>);

    fn models(&self) -> &[Model<T>];

    fn models_mut(&mut self) -> &mut [Model<T>];

    fn to_string(&self) -> Result<Vec<(String, String)>, Self::Error>;
}
