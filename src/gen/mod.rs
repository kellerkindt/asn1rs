pub mod protobuf;
pub mod rust;

pub use self::rust::RustCodeGenerator;

use model::Model;

pub trait Generator<T> {
    type Error;

    fn add_model(&mut self, model: Model<T>);

    fn models(&self) -> &[Model<T>];

    fn models_mut(&mut self) -> &mut [Model<T>];

    fn to_string(&self) -> Result<Vec<(String, String)>, Self::Error>;
}
