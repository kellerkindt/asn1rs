use crate::model::rs::ResolveScope;
use crate::model::{Asn, LiteralValue, Model};
use std::fmt::{Debug, Display, Formatter};

pub trait ResolveState: Clone {
    type SizeType: Display + Debug + Clone + PartialOrd + PartialEq;
    type RangeType: Display + Debug + Clone + PartialOrd + PartialEq;
    type ConstType: Debug + Clone + PartialOrd + PartialEq;
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct Resolved;
impl ResolveState for Resolved {
    type SizeType = usize;
    type RangeType = i64;
    type ConstType = LiteralValue;
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub struct Unresolved;
impl ResolveState for Unresolved {
    type SizeType = LitOrRef<usize>;
    type RangeType = LitOrRef<i64>;
    type ConstType = LitOrRef<LiteralValue>;
}

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum LitOrRef<T> {
    Lit(T),
    Ref(String),
}

impl<T> Default for LitOrRef<T>
where
    T: Default,
{
    fn default() -> Self {
        LitOrRef::Lit(T::default())
    }
}

impl<T> Display for LitOrRef<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LitOrRef::Lit(v) => Display::fmt(v, f),
            LitOrRef::Ref(v) => Display::fmt(v, f),
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Eq)]
pub enum Error {
    FailedToResolveType(String),
    FailedToResolveReference(String),
    FailedToParseLiteral(String),
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FailedToResolveType(name) => {
                write!(f, "Failed to resolve type with name: {}", name)
            }
            Error::FailedToResolveReference(name) => {
                write!(f, "Failed to resolve reference with name: {}", name)
            }
            Error::FailedToParseLiteral(literal) => {
                write!(f, "Failed to parse literal: {}", literal)
            }
        }
    }
}

pub trait Resolver<T> {
    fn resolve(&self, lor: &LitOrRef<T>) -> Result<T, Error>;
}

pub trait TryResolve<T, R: Sized> {
    fn try_resolve(&self, resolver: &impl Resolver<T>) -> Result<R, Error>;
}

impl Model<Asn<Unresolved>> {
    pub fn try_resolve(&self) -> Result<Model<Asn<Resolved>>, Error> {
        ResolveScope::from(self).try_resolve()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::model::Range;
    use crate::model::Type;
    use crate::model::{Definition, Integer, ValueReference};

    #[test]
    fn test_simple_resolve() {
        let mut unresolved = Model::<Asn<Unresolved>> {
            name: "UnresolvedModel".to_string(),
            definitions: vec![Definition(
                "IntegerWithVR".to_string(),
                Type::<Unresolved>::Integer(Integer {
                    range: Range(
                        Some(LitOrRef::Ref("my_min".to_string())),
                        Some(LitOrRef::Ref("my_max".to_string())),
                        true,
                    ),
                    constants: Vec::default(),
                })
                .untagged(),
            )],
            ..Default::default()
        };

        assert_eq!(
            Error::FailedToResolveReference("my_min".to_string()),
            unresolved.try_resolve().unwrap_err()
        );

        unresolved.value_references.push(ValueReference {
            name: "my_min".to_string(),
            role: Type::Integer(Integer::default()).untagged(),
            value: LiteralValue::Integer(123),
        });

        assert_eq!(
            Error::FailedToResolveReference("my_max".to_string()),
            unresolved.try_resolve().unwrap_err()
        );

        unresolved.value_references.push(ValueReference {
            name: "my_max".to_string(),
            role: Type::Integer(Integer::default()).untagged(),
            value: LiteralValue::Integer(456),
        });

        let resolved = unresolved.try_resolve().unwrap();
        assert_eq!(
            &resolved.definitions[..],
            &[Definition(
                "IntegerWithVR".to_string(),
                Type::<Resolved>::Integer(Integer {
                    range: Range(Some(123), Some(456), true),
                    constants: Vec::default(),
                })
                .untagged(),
            )]
        )
    }
}
