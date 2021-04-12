use crate::model::{Asn, Definition, LiteralValue, Model, Type, ValueReference};
use std::fmt::{Debug, Display, Formatter};

pub trait ResolveState: Clone {
    type SizeType: Display + Debug + Clone + PartialOrd + PartialEq;
    type RangeType: Display + Debug + Clone + PartialOrd + PartialEq;
    type ConstType: Debug + Clone + PartialOrd + PartialEq;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Resolved;
impl ResolveState for Resolved {
    type SizeType = usize;
    type RangeType = i64;
    type ConstType = LiteralValue;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Unresolved;
impl ResolveState for Unresolved {
    type SizeType = LitOrRef<usize>;
    type RangeType = LitOrRef<i64>;
    type ConstType = LitOrRef<LiteralValue>;
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
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

#[derive(Debug, PartialOrd, PartialEq)]
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
        let mut model = Model::<Asn<Resolved>> {
            name: self.name.clone(),
            oid: self.oid.clone(),
            imports: self.imports.clone(),
            definitions: Vec::with_capacity(self.definitions.len()),
            value_references: Vec::with_capacity(self.value_references.len()),
        };

        // copy over all value references
        for vr in &self.value_references {
            model.value_references.push(ValueReference {
                name: vr.name.clone(),
                role: vr.role.try_resolve(self)?,
                value: vr.value.clone(),
            })
        }

        for Definition(name, asn) in &self.definitions {
            model
                .definitions
                .push(Definition(name.clone(), asn.try_resolve(self)?))
        }

        Ok(model)
    }
}

impl Resolver<usize> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<usize>) -> Result<usize, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => match self
                .value_references
                .iter()
                .find(|vr| vr.name.eq(name))
                .map(|vr| vr.value.to_integer())
            {
                Some(Some(value)) => Ok(value as usize),
                Some(None) => Err(Error::FailedToParseLiteral(format!("name: {}", name))),
                None => Err(Error::FailedToResolveReference(name.clone())),
            },
        }
    }
}

impl Resolver<i64> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<i64>) -> Result<i64, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(*lit),
            LitOrRef::Ref(name) => match self
                .value_references
                .iter()
                .find(|vr| vr.name.eq(name))
                .map(|vr| vr.value.to_integer())
            {
                Some(Some(value)) => Ok(value),
                Some(None) => Err(Error::FailedToParseLiteral(format!("name: {}", name))),
                None => Err(Error::FailedToResolveReference(name.clone())),
            },
        }
    }
}

impl Resolver<LiteralValue> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<LiteralValue>) -> Result<LiteralValue, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(lit.clone()),
            LitOrRef::Ref(name) => self
                .value_references
                .iter()
                .find(|vr| vr.name.eq(name))
                .map(|vr| vr.value.clone())
                .ok_or_else(|| Error::FailedToResolveReference(name.clone())),
        }
    }
}

impl Resolver<Type<Unresolved>> for Model<Asn<Unresolved>> {
    fn resolve(&self, lor: &LitOrRef<Type<Unresolved>>) -> Result<Type<Unresolved>, Error> {
        match lor {
            LitOrRef::Lit(lit) => Ok(lit.clone()),
            LitOrRef::Ref(name) => self
                .definitions
                .iter()
                .find(|def| def.0.eq(name))
                .map(|def| def.1.r#type.clone())
                .ok_or_else(|| Error::FailedToResolveType(name.clone())),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::model::Integer;
    use crate::model::Range;
    use crate::model::Type;

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
