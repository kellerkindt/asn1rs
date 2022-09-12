/// The object-identifier is described in ITU-T X.680 | ISO/IEC 8824-1:2015
/// in chapter 32. The XML-related definitions as well as'DefinedValue' is
/// ignored by this implementation.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct ObjectIdentifier(pub Vec<ObjectIdentifierComponent>);

impl ObjectIdentifier {
    pub fn iter(&self) -> impl Iterator<Item = &ObjectIdentifierComponent> {
        self.0.iter()
    }
}

/// The object-identifier is described in ITU-T X.680 | ISO/IEC 8824-1:2015
/// in chapter 32. The XML-related definitions as well as'DefinedValue' is
/// ignored by this implementation.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum ObjectIdentifierComponent {
    NameForm(String),
    NumberForm(u64),
    NameAndNumberForm(String, u64),
}
