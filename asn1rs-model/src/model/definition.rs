use crate::model::{Asn, Rust, Tag, TagProperty};

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
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

    fn with_tag_opt(self, tag: Option<Tag>) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.with_tag_opt(tag))
    }

    fn with_tag(self, tag: Tag) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.with_tag(tag))
    }

    fn without_tag(self) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.without_tag())
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

    fn with_tag_opt(self, tag: Option<Tag>) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.with_tag_opt(tag))
    }

    fn with_tag(self, tag: Tag) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.with_tag(tag))
    }

    fn without_tag(self) -> Self
    where
        Self: Sized,
    {
        Self(self.0, self.1.without_tag())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::rust::PlainEnum;
    use crate::model::tag::tests::test_property;
    use crate::model::Type;

    #[test]
    pub fn test_tag_property_definition_asn() {
        test_property(Definition(
            String::default(),
            Asn::from(Type::unconstrained_integer()),
        ));
    }

    #[test]
    pub fn test_tag_property_definition_rust() {
        test_property(Definition(
            String::default(),
            Rust::Enum(PlainEnum::from_names(Some("Variant").into_iter())),
        ));
    }
}
