pub mod protobuf;
pub mod rust;

pub use self::rust::RustCodeGenerator;

use model::Model;

pub trait Generator {
    fn add_model(&mut self, model: Model);

    fn models(&self) -> &[Model];

    fn models_mut(&mut self) -> &mut [Model];

    fn to_string(&self) -> Vec<(String, String)>;
}
