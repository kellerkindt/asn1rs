use crate::model::{Asn, Rust, Tag, TagProperty};

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Definition<T>(pub String, pub T);

impl<T> Definition<T> {
    #[cfg(test)]
    pub fn new<I: ToString>(name: I, value: T) -> Self {
        Definition(name.to_string(), value)
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn value(&self) -> &T {
        &self.1
    }
}

impl TagProperty for Definition<Asn> {
    fn tag(&self) -> Option<Tag> {
        self.1.tag()
    }

    fn set_tag(&mut self, tag: Tag) {
        self.1.set_tag(tag)
    }

    fn reset_tag(&mut self) {
        self.1.reset_tag()
    }
}

impl TagProperty for Definition<Rust> {
    fn tag(&self) -> Option<Tag> {
        self.1.tag()
    }

    fn set_tag(&mut self, tag: Tag) {
        self.1.set_tag(tag)
    }

    fn reset_tag(&mut self) {
        self.1.reset_tag()
    }
}
